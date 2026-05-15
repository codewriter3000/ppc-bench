//! PPC (Gekko/Broadway) interpreter & assembler.
//!
//! Semantics are ported from the Dolphin emulator (GPL-2.0+).
//! Sources of reference (Dolphin master @ Source/Core/Core/PowerPC):
//!   - `Gekko.h`           — UGeckoInstruction union, SPR/MSR/XER/FPSCR bit layouts
//!   - `PPCTables.cpp/h`   — primary + extended opcode tables, GekkoOPTemplate
//!   - `PowerPC.h`         — PowerPCState
//!   - `Interpreter_*.cpp` — per-group instruction semantics
//!
//! The engine is single-threaded; the Tauri layer wraps it in a Mutex.

pub mod assembler;
pub mod binary_loader;
pub mod commands;
pub mod disassembler;
pub mod dol;
pub mod inst;
pub mod instructions;
pub mod interpreter;
pub mod memory;
pub mod snapshot;
pub mod state;
pub mod tables;

pub use state::PPCEngine;
