//! PPC-Bench root library.
//!
//! Hosts the Tauri runtime and exposes the [`ppc_engine`] module to the
//! frontend via the commands defined in [`ppc_engine::commands`].

pub mod dolphin;
pub mod gdb_client;
pub mod ppc_engine;
pub mod settings;

use std::net::TcpStream;
use std::sync::atomic::AtomicBool;
use std::sync::{Arc, Mutex};

use dolphin::DolphinSession;
use gdb_client::GdbClient;
use ppc_engine::PPCEngine;

/// Tauri-managed handle around the (mutex-guarded) PPC engine. All Tauri
/// commands acquire the lock for the duration of their call.
pub struct EngineState(pub Mutex<PPCEngine>);

/// Tauri-managed Dolphin child process handle for emulator mode.
pub struct DolphinState(pub Arc<Mutex<Option<DolphinSession>>>);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GdbExecutionState {
    Disconnected,
    Paused,
    Running,
}

#[derive(Clone)]
pub struct GdbConnection {
    pub client: Arc<Mutex<GdbClient>>,
    pub interrupt: Arc<Mutex<TcpStream>>,
}

pub struct GdbState {
    pub connection: Arc<Mutex<Option<GdbConnection>>>,
    pub execution: Arc<Mutex<GdbExecutionState>>,
    pub interrupt_requested: Arc<AtomicBool>,
}

impl Default for GdbState {
    fn default() -> Self {
        Self {
            connection: Arc::new(Mutex::new(None)),
            execution: Arc::new(Mutex::new(GdbExecutionState::Disconnected)),
            interrupt_requested: Arc::new(AtomicBool::new(false)),
        }
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .manage(EngineState(Mutex::new(PPCEngine::new())))
        .manage(DolphinState(Arc::new(Mutex::new(None))))
        .manage(GdbState::default())
        .invoke_handler(tauri::generate_handler![
            ppc_engine::commands::assemble,
            ppc_engine::commands::disassemble,
            ppc_engine::commands::load_program,
            ppc_engine::commands::load_binary,
            ppc_engine::commands::step,
            ppc_engine::commands::run_until,
            ppc_engine::commands::launch_with_emulator,
            ppc_engine::commands::connect_gdb,
            ppc_engine::commands::capture_emulator_snapshot,
            ppc_engine::commands::emulator_continue,
            ppc_engine::commands::emulator_step,
            ppc_engine::commands::emulator_break,
            ppc_engine::commands::emulator_set_breakpoint,
            ppc_engine::commands::emulator_clear_breakpoint,
            ppc_engine::commands::emulator_read_memory,
            ppc_engine::commands::stop_emulator,
            ppc_engine::commands::probe_emulator,
            ppc_engine::commands::reset,
            ppc_engine::commands::get_state,
            ppc_engine::commands::set_breakpoint,
            ppc_engine::commands::clear_breakpoint,
            ppc_engine::commands::get_breakpoints,
            ppc_engine::commands::read_memory,
            ppc_engine::commands::get_call_stack,
            ppc_engine::commands::get_trace,
            ppc_engine::commands::get_symbols,
            settings::load_settings,
            settings::save_settings,
            settings::pick_dolphin_path,
        ])
        .run(tauri::generate_context!())
        .expect("error while running PPC-Bench");
}
