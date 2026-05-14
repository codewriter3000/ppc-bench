//! PPC-Bench root library.
//!
//! Hosts the Tauri runtime and exposes the [`ppc_engine`] module to the
//! frontend via the commands defined in [`ppc_engine::commands`].

pub mod ppc_engine;

use std::sync::Mutex;

use ppc_engine::PPCEngine;

/// Tauri-managed handle around the (mutex-guarded) PPC engine. All Tauri
/// commands acquire the lock for the duration of their call.
pub struct EngineState(pub Mutex<PPCEngine>);

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .manage(EngineState(Mutex::new(PPCEngine::new())))
        .invoke_handler(tauri::generate_handler![
            ppc_engine::commands::assemble,
            ppc_engine::commands::disassemble,
            ppc_engine::commands::load_program,
            ppc_engine::commands::step,
            ppc_engine::commands::run_until,
            ppc_engine::commands::reset,
            ppc_engine::commands::get_state,
            ppc_engine::commands::set_breakpoint,
            ppc_engine::commands::clear_breakpoint,
            ppc_engine::commands::get_breakpoints,
            ppc_engine::commands::read_memory,
            ppc_engine::commands::get_call_stack,
            ppc_engine::commands::get_trace,
            ppc_engine::commands::get_symbols,
        ])
        .run(tauri::generate_context!())
        .expect("error while running PPC-Bench");
}
