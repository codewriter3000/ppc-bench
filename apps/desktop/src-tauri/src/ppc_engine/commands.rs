//! Tauri command handlers — the JS↔Rust boundary.

use serde::{Deserialize, Serialize};
use tauri::State;

use super::assembler::{assemble as do_assemble, AssembleResult};
use super::disassembler::{disassemble as do_disassemble, DisasmLine};
use super::interpreter::{run_until as engine_run, step as engine_step};
use super::memory::BASE_ADDR;
use super::snapshot::{to_snapshot, CallFrame, MachineStateSnapshot};
use super::state::{HaltReason, TraceEntry};
use crate::EngineState;

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

fn lock_err() -> String { "engine mutex poisoned".to_string() }

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
    engine.cpu.pc = BASE_ADDR;
    Ok(to_snapshot(&engine))
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

fn sorted_breakpoints(set: &std::collections::HashSet<u32>) -> Vec<u32> {
    let mut v: Vec<u32> = set.iter().copied().collect();
    v.sort_unstable();
    v
}
