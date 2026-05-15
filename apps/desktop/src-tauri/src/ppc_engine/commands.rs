//! Tauri command handlers — the JS↔Rust boundary.

use std::collections::HashSet;
use std::sync::atomic::Ordering;
use std::thread;
use std::time::Duration;

use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter, State};

use super::assembler::{assemble as do_assemble, AssembleResult};
use super::binary_loader::load_binary as do_load_binary;
use super::disassembler::{disassemble as do_disassemble, DisasmLine};
use super::dol::generate_dol;
use super::interpreter::{run_until as engine_run, step as engine_step};
use super::memory::BASE_ADDR;
use super::snapshot::{to_snapshot, CallFrame, FPUSnapshot, MachineStateSnapshot, RegisterSnapshot};
use super::state::{HaltReason, LaunchImage, PPCEngine, TraceEntry, WatchpointKind};
use crate::dolphin::{
    default_gdb_port,
    describe_dolphin_exit,
    find_dolphin_with_picker,
    launch_dolphin,
    stop_session,
    write_temp_launch_image,
    DolphinSession,
};
use crate::gdb_client::{
    send_interrupt,
    GdbClient,
    GdbRegisterState,
    StopPacket,
    StopSignal,
    StopWatchpointKind,
};
use crate::settings::load_app_settings;
use crate::{DolphinState, EngineState, GdbConnection, GdbExecutionState, GdbState};

const DOLPHIN_GDB_CONNECT_TIMEOUT: Duration = Duration::from_secs(20);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StepResult {
    pub state: MachineStateSnapshot,
    pub trace: Option<TraceEntry>,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RunResult {
    pub state: MachineStateSnapshot,
    pub steps_executed: u32,
    pub halt_reason: HaltReason,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BinarySection {
    pub name: String,
    pub load_addr: u32,
    pub size: u32,
    pub is_executable: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BinaryLoadResult {
    pub snapshot: MachineStateSnapshot,
    pub disasm_lines: Vec<DisasmLine>,
    pub sections: Vec<BinarySection>,
    pub format: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BinaryLoadProgress {
    pub value: f32,
    pub label: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmulatorLaunchResult {
    pub dol_path: String,
    pub dolphin_path: String,
    pub gdb_port: u16,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum EmulatorStopReason {
    InitialStop,
    Step,
    ManualBreak,
    Breakpoint(u32),
    Watchpoint(EmulatorWatchpointStop),
    Signal(EmulatorSignalStop),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmulatorSignalStop {
    pub signal: u8,
    pub exception_code: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmulatorWatchpointStop {
    pub signal: u8,
    pub kind: WatchpointKind,
    pub address: u32,
    pub exception_code: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmulatorStopResult {
    pub snapshot: MachineStateSnapshot,
    pub reason: EmulatorStopReason,
}

fn lock_err() -> String { "engine mutex poisoned".to_string() }
fn dolphin_lock_err() -> String { "dolphin process mutex poisoned".to_string() }
fn gdb_lock_err() -> String { "gdb state mutex poisoned".to_string() }

fn enrich_dolphin_error(
    dolphin_sessions: &std::sync::Mutex<Option<DolphinSession>>,
    context: &str,
    fallback: String,
) -> String {
    let Ok(mut slot) = dolphin_sessions.lock() else {
        return fallback;
    };

    let detail = match slot.as_mut() {
        Some(session) => match describe_dolphin_exit(session, context) {
            Ok(detail) => detail,
            Err(err) => return format!("{fallback} (failed to inspect Dolphin process: {err})"),
        },
        None => return fallback,
    };

    if let Some(detail) = detail {
        *slot = None;
        return format!("{detail} GDB detail: {fallback}");
    }

    fallback
}

fn build_emulator_launch_image(engine: &PPCEngine) -> Result<(Vec<u8>, String), String> {
    if let Some(image) = &engine.launch_image {
        return Ok(match image {
            LaunchImage::SyntheticProgram { bytes, load_addr } => (
                generate_dol(bytes, *load_addr, engine.cpu.pc),
                "dol".to_string(),
            ),
            LaunchImage::OriginalBinary { bytes, extension } => (bytes.clone(), extension.clone()),
        });
    }

    if engine.program_end <= BASE_ADDR {
        return Err("no program is loaded; load assembled bytes before starting Dolphin".to_string());
    }

    let length = engine.program_end - BASE_ADDR;
    let bytes = engine
        .mem
        .read_bytes(BASE_ADDR, length)
        .map_err(|err| err.to_string())?
        .to_vec();

    Ok((generate_dol(&bytes, BASE_ADDR, engine.cpu.pc), "dol".to_string()))
}

fn binary_extension(format: &str) -> String {
    if format.eq_ignore_ascii_case("elf") {
        "elf".to_string()
    } else {
        "dol".to_string()
    }
}

fn emit_binary_load_progress(app: &AppHandle, value: f32, label: impl Into<String>) {
    let _ = app.emit(
        "binary-load-progress",
        BinaryLoadProgress {
            value,
            label: label.into(),
        },
    );
}

#[derive(Clone)]
struct EmulatorMetadata {
    breakpoints: Vec<u32>,
    symbols: Vec<(String, u32)>,
    program_end: u32,
    step_count: u64,
}

#[tauri::command]
pub fn assemble(state: State<'_, EngineState>, source: String) -> Result<AssembleResult, String> {
    let _ = state; // not used here; reserved for future "assemble & load" fast path
    Ok(do_assemble(&source))
}

#[tauri::command]
pub fn disassemble(bytes: Vec<u8>, base_addr: u32) -> Result<Vec<DisasmLine>, String> {
    Ok(do_disassemble(&bytes, base_addr))
}

#[tauri::command]
pub fn load_program(
    state: State<'_, EngineState>,
    bytes: Vec<u8>,
    symbols: Vec<(String, u32)>,
) -> Result<MachineStateSnapshot, String> {
    let mut engine = state.0.lock().map_err(|_| lock_err())?;
    engine.reset();
    engine.symbols = symbols;
    engine.mem.write_bytes(BASE_ADDR, &bytes).map_err(|e| e.to_string())?;
    engine.program_end = BASE_ADDR.wrapping_add(bytes.len() as u32);
    engine.launch_image = Some(LaunchImage::SyntheticProgram {
        bytes: bytes.clone(),
        load_addr: BASE_ADDR,
    });
    engine.cpu.pc = BASE_ADDR;
    Ok(to_snapshot(&engine))
}

#[tauri::command]
pub fn load_binary(
    app: AppHandle,
    state: State<'_, EngineState>,
    bytes: Vec<u8>,
) -> Result<BinaryLoadResult, String> {
    emit_binary_load_progress(&app, 0.18, "Parsing binary sections");
    let binary = do_load_binary(&bytes)?;
    let mut engine = state.0.lock().map_err(|_| lock_err())?;
    engine.reset();

    let mut disasm_lines = Vec::new();
    let total_sections = binary.sections.len().max(1) as f32;
    for (index, section) in binary.sections.iter().enumerate() {
        engine
            .mem
            .write_bytes(section.load_addr, &section.bytes)
            .map_err(|err| err.to_string())?;

        emit_binary_load_progress(
            &app,
            0.25 + ((index as f32) / total_sections) * 0.6,
            format!("Loading {} at 0x{:08X}", section.name, section.load_addr),
        );

        if section.is_executable && section.disasm_len != 0 {
            disasm_lines.extend(do_disassemble(
                &section.bytes[..section.disasm_len as usize],
                section.load_addr,
            ));
        }
    }

    disasm_lines.sort_by_key(|line| line.address);
    engine.program_end = binary.program_end;
    engine.launch_image = Some(LaunchImage::OriginalBinary {
        bytes: bytes.clone(),
        extension: binary_extension(&binary.format),
    });
    engine.cpu.pc = binary.entry_point;
    emit_binary_load_progress(&app, 0.92, "Finalizing binary load");

    let sections = binary
        .sections
        .iter()
        .map(|section| BinarySection {
            name: section.name.clone(),
            load_addr: section.load_addr,
            size: section.bytes.len() as u32,
            is_executable: section.is_executable,
        })
        .collect();

    Ok(BinaryLoadResult {
        snapshot: to_snapshot(&engine),
        disasm_lines,
        sections,
        format: binary.format,
    })
}

#[tauri::command]
pub fn step(state: State<'_, EngineState>) -> Result<StepResult, String> {
    let mut engine = state.0.lock().map_err(|_| lock_err())?;
    let (trace, error) = match engine_step(&mut engine) {
        Ok(r) => (Some(r.trace), None),
        Err(reason) => (None, Some(format!("{:?}", reason))),
    };
    Ok(StepResult { state: to_snapshot(&engine), trace, error })
}

#[tauri::command]
pub fn run_until(state: State<'_, EngineState>, max_steps: u32) -> Result<RunResult, String> {
    let mut engine = state.0.lock().map_err(|_| lock_err())?;
    let cap = if max_steps == 0 { 1_000_000 } else { max_steps };
    let (n, reason) = engine_run(&mut engine, cap);
    Ok(RunResult {
        state: to_snapshot(&engine),
        steps_executed: n,
        halt_reason: reason,
    })
}

#[tauri::command]
pub fn launch_with_emulator(
    app: AppHandle,
    state: State<'_, EngineState>,
    dolphin_state: State<'_, DolphinState>,
) -> Result<EmulatorLaunchResult, String> {
    let (launch_bytes, launch_extension) = {
        let engine = state.0.lock().map_err(|_| lock_err())?;
        build_emulator_launch_image(&engine)?
    };

    let settings = load_app_settings(&app)?;
    let dolphin_path = find_dolphin_with_picker(&app, settings.dolphin_path.as_deref())?;
    let gdb_port = default_gdb_port();
    let dol_path = write_temp_launch_image(&launch_bytes, &launch_extension)?;

    let mut slot = dolphin_state.0.lock().map_err(|_| dolphin_lock_err())?;
    if let Some(mut session) = slot.take() {
        stop_session(&mut session)?;
    }

    let session = launch_dolphin(&dolphin_path, &dol_path, gdb_port, settings.dolphin_enable_mmu)?;
    *slot = Some(session);

    Ok(EmulatorLaunchResult {
        dol_path: dol_path.display().to_string(),
        dolphin_path: dolphin_path.display().to_string(),
        gdb_port,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn emulator_launch_uses_original_binary_bytes_when_present() {
        let mut engine = PPCEngine::new();
        engine.program_end = BASE_ADDR + 4;
        engine.launch_image = Some(LaunchImage::OriginalBinary {
            bytes: vec![0x01, 0x02, 0x03, 0x04],
            extension: "dol".to_string(),
        });

        let (bytes, extension) = build_emulator_launch_image(&engine).expect("launch image should build");

        assert_eq!(bytes, vec![0x01, 0x02, 0x03, 0x04]);
        assert_eq!(extension, "dol");
    }

    #[test]
    fn emulator_launch_synthesizes_dol_for_assembled_programs() {
        let mut engine = PPCEngine::new();
        engine.cpu.pc = BASE_ADDR + 4;
        engine.launch_image = Some(LaunchImage::SyntheticProgram {
            bytes: vec![0x60, 0x00, 0x00, 0x00, 0x4E, 0x80, 0x00, 0x20],
            load_addr: BASE_ADDR,
        });

        let (bytes, extension) = build_emulator_launch_image(&engine).expect("launch image should build");

        assert_eq!(extension, "dol");
        assert_eq!(&bytes[0x48..0x4C], &BASE_ADDR.to_be_bytes());
        assert_eq!(&bytes[0xE0..0xE4], &(BASE_ADDR + 4).to_be_bytes());
    }
}

#[tauri::command]
pub fn connect_gdb(
    state: State<'_, EngineState>,
    dolphin_state: State<'_, DolphinState>,
    gdb_state: State<'_, GdbState>,
    port: u16,
) -> Result<EmulatorStopResult, String> {
    let mut client = GdbClient::connect(port, DOLPHIN_GDB_CONNECT_TIMEOUT).map_err(|err| {
        enrich_dolphin_error(
            dolphin_state.0.as_ref(),
            "Dolphin exited before the GDB stub became available.",
            err,
        )
    })?;
    let interrupt = client.interrupt_clone()?;
    let _ = client.query_stop_reason().map_err(|err| {
        enrich_dolphin_error(
            dolphin_state.0.as_ref(),
            "Dolphin exited before the initial debugger stop was received.",
            err,
        )
    })?;

    let metadata = emulator_metadata(&state)?;
    let snapshot = capture_snapshot_from_client(&mut client, &metadata, HaltReason::Breakpoint(0))?;

    let connection = GdbConnection {
        client: std::sync::Arc::new(std::sync::Mutex::new(client)),
        interrupt: std::sync::Arc::new(std::sync::Mutex::new(interrupt)),
    };

    *gdb_state.connection.lock().map_err(|_| gdb_lock_err())? = Some(connection);
    *gdb_state.execution.lock().map_err(|_| gdb_lock_err())? = GdbExecutionState::Paused;
    gdb_state.interrupt_requested.store(false, Ordering::SeqCst);

    Ok(EmulatorStopResult {
        snapshot,
        reason: EmulatorStopReason::InitialStop,
    })
}

#[tauri::command]
pub fn capture_emulator_snapshot(
    state: State<'_, EngineState>,
    gdb_state: State<'_, GdbState>,
) -> Result<MachineStateSnapshot, String> {
    ensure_not_running(&gdb_state)?;
    let metadata = emulator_metadata(&state)?;
    with_gdb_client(&gdb_state, |client| {
        capture_snapshot_from_client(client, &metadata, HaltReason::Breakpoint(0))
    })
}

#[tauri::command]
pub fn emulator_continue(
    app: AppHandle,
    state: State<'_, EngineState>,
    dolphin_state: State<'_, DolphinState>,
    gdb_state: State<'_, GdbState>,
) -> Result<(), String> {
    let connection = current_connection(&gdb_state)?;
    ensure_not_running(&gdb_state)?;

    {
        let mut client = connection.client.lock().map_err(|_| gdb_lock_err())?;
        client.send_continue()?;
    }

    *gdb_state.execution.lock().map_err(|_| gdb_lock_err())? = GdbExecutionState::Running;
    gdb_state.interrupt_requested.store(false, Ordering::SeqCst);

    let metadata = emulator_metadata(&state)?;
    let execution = gdb_state.execution.clone();
    let interrupt_requested = gdb_state.interrupt_requested.clone();
    let dolphin_sessions = dolphin_state.0.clone();

    thread::spawn(move || {
        let emit_result = (|| -> Result<EmulatorStopResult, String> {
            let mut client = connection.client.lock().map_err(|_| gdb_lock_err())?;
            match client.wait_for_stop()? {
                StopPacket::Exit(code) => Err(format!("Dolphin process exited through GDB with code {code}")),
                StopPacket::Reply(payload) => Err(format!("unexpected GDB stop reply: {payload}")),
                StopPacket::Signal(stop) => {
                    let interrupted = interrupt_requested.swap(false, Ordering::SeqCst);
                    let reason = emulator_stop_reason_from_signal(&stop, interrupted);
                    let halt_reason = halt_reason_from_emulator_stop(&reason);

                    let snapshot = capture_snapshot_from_client(&mut client, &metadata, halt_reason)?;
                    let reason = match reason {
                        EmulatorStopReason::Breakpoint(0) => {
                            EmulatorStopReason::Breakpoint(snapshot.registers.pc)
                        }
                        other => other,
                    };
                    Ok(EmulatorStopResult { snapshot, reason })
                }
            }
        })();

        let emit_result = match emit_result {
            Ok(result) => Ok(result),
            Err(message) => Err(enrich_dolphin_error(
                dolphin_sessions.as_ref(),
                "Dolphin exited while the program was running.",
                message,
            )),
        };

        if let Ok(mut state) = execution.lock() {
            *state = if emit_result.is_ok() {
                GdbExecutionState::Paused
            } else {
                GdbExecutionState::Disconnected
            };
        }

        match emit_result {
            Ok(result) => {
                let _ = app.emit("emulator-stopped", result);
            }
            Err(message) => {
                let _ = app.emit("emulator-error", message);
            }
        }
    });

    Ok(())
}

#[tauri::command]
pub fn emulator_step(
    state: State<'_, EngineState>,
    dolphin_state: State<'_, DolphinState>,
    gdb_state: State<'_, GdbState>,
) -> Result<EmulatorStopResult, String> {
    ensure_not_running(&gdb_state)?;
    let metadata = emulator_metadata(&state)?;
    let result = with_gdb_client(&gdb_state, |client| {
        let stop = client.step()?;
        let reason = match &stop {
            StopPacket::Signal(signal) => emulator_stop_reason_from_signal(signal, false),
            StopPacket::Exit(code) => return Err(format!("Dolphin process exited through GDB with code {code}")),
            StopPacket::Reply(payload) => return Err(format!("unexpected GDB stop reply: {payload}")),
        };
        let halt_reason = halt_reason_from_emulator_stop(&reason);
        let snapshot = capture_snapshot_from_client(client, &metadata, halt_reason)?;
        let reason = match stop {
            StopPacket::Signal(signal) if signal.watchpoint.is_none() && signal.signal == 5 => EmulatorStopReason::Step,
            StopPacket::Signal(_) => reason,
            StopPacket::Exit(_) | StopPacket::Reply(_) => unreachable!(),
        };
        Ok(EmulatorStopResult {
            snapshot,
            reason,
        })
    })
    .map_err(|err| {
        enrich_dolphin_error(
            dolphin_state.0.as_ref(),
            "Dolphin exited while single-stepping.",
            err,
        )
    })?;

    *gdb_state.execution.lock().map_err(|_| gdb_lock_err())? = GdbExecutionState::Paused;
    Ok(result)
}

#[tauri::command]
pub fn emulator_break(gdb_state: State<'_, GdbState>) -> Result<(), String> {
    {
        let execution = gdb_state.execution.lock().map_err(|_| gdb_lock_err())?;
        if *execution != GdbExecutionState::Running {
            return Err("emulator is not currently running".to_string());
        }
    }

    let connection = current_connection(&gdb_state)?;
    gdb_state.interrupt_requested.store(true, Ordering::SeqCst);
    let mut stream = connection.interrupt.lock().map_err(|_| gdb_lock_err())?;
    send_interrupt(&mut stream)
}

#[tauri::command]
pub fn emulator_set_breakpoint(
    state: State<'_, EngineState>,
    gdb_state: State<'_, GdbState>,
    address: u32,
) -> Result<Vec<u32>, String> {
    ensure_not_running(&gdb_state)?;
    with_gdb_client(&gdb_state, |client| client.set_breakpoint(address))?;

    let mut engine = state.0.lock().map_err(|_| lock_err())?;
    engine.breakpoints.insert(address);
    Ok(sorted_breakpoints(&engine.breakpoints))
}

#[tauri::command]
pub fn emulator_clear_breakpoint(
    state: State<'_, EngineState>,
    gdb_state: State<'_, GdbState>,
    address: u32,
) -> Result<Vec<u32>, String> {
    ensure_not_running(&gdb_state)?;
    with_gdb_client(&gdb_state, |client| client.clear_breakpoint(address))?;

    let mut engine = state.0.lock().map_err(|_| lock_err())?;
    engine.breakpoints.remove(&address);
    Ok(sorted_breakpoints(&engine.breakpoints))
}

#[tauri::command]
pub fn emulator_read_memory(
    gdb_state: State<'_, GdbState>,
    address: u32,
    length: u32,
) -> Result<Vec<u8>, String> {
    ensure_not_running(&gdb_state)?;
    with_gdb_client(&gdb_state, |client| client.read_memory(address, length))
}

#[tauri::command]
pub fn stop_emulator(
    dolphin_state: State<'_, DolphinState>,
    gdb_state: State<'_, GdbState>,
) -> Result<(), String> {
    let mut slot = dolphin_state.0.lock().map_err(|_| dolphin_lock_err())?;
    if let Some(mut session) = slot.take() {
        stop_session(&mut session)?;
    }

    disconnect_gdb_state(gdb_state.inner())?;
    Ok(())
}

#[tauri::command]
pub fn probe_emulator(
    dolphin_state: State<'_, DolphinState>,
    gdb_state: State<'_, GdbState>,
) -> Result<Option<String>, String> {
    let mut slot = dolphin_state.0.lock().map_err(|_| dolphin_lock_err())?;
    let detail = match slot.as_mut() {
        Some(session) => describe_dolphin_exit(session, "Dolphin exited while the debugger session was idle.")?,
        None => None,
    };

    if let Some(detail) = detail {
        *slot = None;
        disconnect_gdb_state(gdb_state.inner())?;
        return Ok(Some(detail));
    }

    Ok(None)
}

#[tauri::command]
pub fn reset(state: State<'_, EngineState>) -> Result<MachineStateSnapshot, String> {
    let mut engine = state.0.lock().map_err(|_| lock_err())?;
    engine.reset();
    Ok(to_snapshot(&engine))
}

#[tauri::command]
pub fn get_state(state: State<'_, EngineState>) -> Result<MachineStateSnapshot, String> {
    let engine = state.0.lock().map_err(|_| lock_err())?;
    Ok(to_snapshot(&engine))
}

#[tauri::command]
pub fn set_breakpoint(state: State<'_, EngineState>, address: u32) -> Result<Vec<u32>, String> {
    let mut engine = state.0.lock().map_err(|_| lock_err())?;
    engine.breakpoints.insert(address);
    Ok(sorted_breakpoints(&engine.breakpoints))
}

#[tauri::command]
pub fn clear_breakpoint(state: State<'_, EngineState>, address: u32) -> Result<Vec<u32>, String> {
    let mut engine = state.0.lock().map_err(|_| lock_err())?;
    engine.breakpoints.remove(&address);
    Ok(sorted_breakpoints(&engine.breakpoints))
}

#[tauri::command]
pub fn get_breakpoints(state: State<'_, EngineState>) -> Result<Vec<u32>, String> {
    let engine = state.0.lock().map_err(|_| lock_err())?;
    Ok(sorted_breakpoints(&engine.breakpoints))
}

#[tauri::command]
pub fn read_memory(
    state: State<'_, EngineState>,
    address: u32,
    length: u32,
) -> Result<Vec<u8>, String> {
    let engine = state.0.lock().map_err(|_| lock_err())?;
    engine
        .mem
        .read_bytes(address, length)
        .map(|s| s.to_vec())
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_call_stack(state: State<'_, EngineState>) -> Result<Vec<CallFrame>, String> {
    let engine = state.0.lock().map_err(|_| lock_err())?;
    Ok(engine
        .call_stack
        .iter()
        .map(|f| CallFrame {
            call_site: f.call_site,
            return_to: f.return_to,
            symbol: f.symbol.clone(),
        })
        .collect())
}

#[tauri::command]
pub fn get_trace(state: State<'_, EngineState>, max: u32) -> Result<Vec<TraceEntry>, String> {
    let engine = state.0.lock().map_err(|_| lock_err())?;
    let take = if max == 0 { engine.trace.len() } else { max as usize };
    let n = engine.trace.len().min(take);
    let start = engine.trace.len() - n;
    Ok(engine.trace.iter().skip(start).cloned().collect())
}

#[tauri::command]
pub fn get_symbols(state: State<'_, EngineState>) -> Result<Vec<(String, u32)>, String> {
    let engine = state.0.lock().map_err(|_| lock_err())?;
    Ok(engine.symbols.clone())
}

fn current_connection(gdb_state: &State<'_, GdbState>) -> Result<GdbConnection, String> {
    gdb_state
        .connection
        .lock()
        .map_err(|_| gdb_lock_err())?
        .as_ref()
        .cloned()
        .ok_or_else(|| "no active GDB connection".to_string())
}

fn disconnect_gdb_state(gdb_state: &GdbState) -> Result<(), String> {
    *gdb_state.connection.lock().map_err(|_| gdb_lock_err())? = None;
    *gdb_state.execution.lock().map_err(|_| gdb_lock_err())? = GdbExecutionState::Disconnected;
    gdb_state.interrupt_requested.store(false, Ordering::SeqCst);
    Ok(())
}

fn with_gdb_client<T>(
    gdb_state: &State<'_, GdbState>,
    f: impl FnOnce(&mut GdbClient) -> Result<T, String>,
) -> Result<T, String> {
    let connection = current_connection(gdb_state)?;
    let mut client = connection.client.lock().map_err(|_| gdb_lock_err())?;
    f(&mut client)
}

fn ensure_not_running(gdb_state: &State<'_, GdbState>) -> Result<(), String> {
    let execution = gdb_state.execution.lock().map_err(|_| gdb_lock_err())?;
    if *execution == GdbExecutionState::Running {
        return Err("emulator is running; pause it before inspecting state".to_string());
    }
    if *execution == GdbExecutionState::Disconnected {
        return Err("no active GDB connection".to_string());
    }
    Ok(())
}

fn map_watchpoint_kind(kind: StopWatchpointKind) -> WatchpointKind {
    match kind {
        StopWatchpointKind::Write => WatchpointKind::Write,
        StopWatchpointKind::Read => WatchpointKind::Read,
        StopWatchpointKind::Access => WatchpointKind::Access,
    }
}

fn emulator_stop_reason_from_signal(stop: &StopSignal, interrupted: bool) -> EmulatorStopReason {
    if interrupted {
        return EmulatorStopReason::ManualBreak;
    }

    if let Some(watchpoint) = &stop.watchpoint {
        return EmulatorStopReason::Watchpoint(EmulatorWatchpointStop {
            signal: stop.signal,
            kind: map_watchpoint_kind(watchpoint.kind),
            address: watchpoint.address,
            exception_code: stop.exception_code.clone(),
        });
    }

    if stop.signal == 5 {
        return EmulatorStopReason::Breakpoint(stop.pc.unwrap_or(0));
    }

    EmulatorStopReason::Signal(EmulatorSignalStop {
        signal: stop.signal,
        exception_code: stop.exception_code.clone(),
    })
}

fn halt_reason_from_emulator_stop(reason: &EmulatorStopReason) -> HaltReason {
    match reason {
        EmulatorStopReason::InitialStop | EmulatorStopReason::Step | EmulatorStopReason::ManualBreak => {
            HaltReason::Breakpoint(0)
        }
        EmulatorStopReason::Breakpoint(addr) => HaltReason::Breakpoint(*addr),
        EmulatorStopReason::Watchpoint(stop) => HaltReason::Watchpoint {
            kind: stop.kind,
            address: stop.address,
        },
        EmulatorStopReason::Signal(stop) => HaltReason::Signal {
            signal: stop.signal,
            exception_code: stop.exception_code.clone(),
        },
    }
}

fn emulator_metadata(state: &State<'_, EngineState>) -> Result<EmulatorMetadata, String> {
    let engine = state.0.lock().map_err(|_| lock_err())?;
    Ok(EmulatorMetadata {
        breakpoints: sorted_breakpoints(&engine.breakpoints),
        symbols: engine.symbols.clone(),
        program_end: engine.program_end,
        step_count: engine.step_count,
    })
}

fn capture_snapshot_from_client(
    client: &mut GdbClient,
    metadata: &EmulatorMetadata,
    halt_reason: HaltReason,
) -> Result<MachineStateSnapshot, String> {
    let regs = client.read_registers()?;
    let call_stack = unwind_call_stack(client, &regs, metadata);
    Ok(snapshot_from_registers(regs, metadata, halt_reason, call_stack))
}

fn snapshot_from_registers(
    regs: GdbRegisterState,
    metadata: &EmulatorMetadata,
    halt_reason: HaltReason,
    call_stack: Vec<CallFrame>,
) -> MachineStateSnapshot {
    let halt_reason = match halt_reason {
        HaltReason::Breakpoint(0) => HaltReason::Breakpoint(regs.pc),
        other => other,
    };

    MachineStateSnapshot {
        registers: RegisterSnapshot {
            gpr: regs.gpr,
            pc: regs.pc,
            lr: regs.lr,
            ctr: regs.ctr,
            xer: regs.xer,
            cr: regs.cr,
            msr: regs.msr,
            changed_gpr: Vec::new(),
            gqr: [0; 8],
        },
        fpu: FPUSnapshot {
            fpr: regs.fpr,
            fpscr: regs.fpscr,
            changed_fpr: Vec::new(),
        },
        step_count: metadata.step_count,
        halted: true,
        halt_reason,
        breakpoints: metadata.breakpoints.clone(),
        call_stack,
        symbols: metadata.symbols.clone(),
        program_end: metadata.program_end,
        last_writes: Vec::new(),
    }
}

fn unwind_call_stack(
    client: &mut GdbClient,
    regs: &GdbRegisterState,
    metadata: &EmulatorMetadata,
) -> Vec<CallFrame> {
    const MAX_FRAMES: usize = 16;

    let mut frames = Vec::new();
    let mut seen_stack_ptrs = HashSet::new();

    let initial_return = regs.lr & !0x3;
    if let Some(frame) = call_frame_from_return(initial_return, metadata) {
        frames.push(frame);
    }

    let mut stack_ptr = regs.gpr[1];
    for _ in 0..MAX_FRAMES {
        if stack_ptr == 0 || !seen_stack_ptrs.insert(stack_ptr) {
            break;
        }

        let Ok(caller_stack_ptr) = read_be_u32(client, stack_ptr) else {
            break;
        };
        if caller_stack_ptr <= stack_ptr {
            break;
        }

        let Ok(saved_lr) = read_be_u32(client, caller_stack_ptr.wrapping_add(4)) else {
            stack_ptr = caller_stack_ptr;
            continue;
        };
        let return_to = saved_lr & !0x3;
        if let Some(frame) = call_frame_from_return(return_to, metadata) {
            let duplicate = frames.iter().any(|existing| existing.return_to == frame.return_to);
            if !duplicate {
                frames.push(frame);
            }
        }

        stack_ptr = caller_stack_ptr;
    }

    frames
}

fn read_be_u32(client: &mut GdbClient, address: u32) -> Result<u32, String> {
    let bytes = client.read_memory(address, 4)?;
    if bytes.len() != 4 {
        return Err(format!("expected 4 bytes at 0x{address:08X}, received {}", bytes.len()));
    }

    Ok(u32::from_be_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]))
}

fn call_frame_from_return(return_to: u32, metadata: &EmulatorMetadata) -> Option<CallFrame> {
    if return_to < BASE_ADDR || return_to >= metadata.program_end {
        return None;
    }

    Some(CallFrame {
        call_site: return_to.wrapping_sub(4),
        return_to,
        symbol: resolve_symbol_for_address(&metadata.symbols, return_to),
    })
}

fn resolve_symbol_for_address(symbols: &[(String, u32)], address: u32) -> Option<String> {
    let (name, base) = symbols
        .iter()
        .filter(|(_, symbol_addr)| *symbol_addr <= address)
        .max_by_key(|(_, symbol_addr)| *symbol_addr)?;
    let offset = address.wrapping_sub(*base);
    Some(if offset == 0 {
        name.clone()
    } else {
        format!("{name}+0x{offset:X}")
    })
}

fn sorted_breakpoints(set: &std::collections::HashSet<u32>) -> Vec<u32> {
    let mut v: Vec<u32> = set.iter().copied().collect();
    v.sort_unstable();
    v
}
