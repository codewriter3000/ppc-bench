//! Serializable snapshots of engine state for the Tauri frontend.
//!
//! Field names + shapes mirror `packages/kernel/src/contracts.ts`.

use serde::{Deserialize, Serialize};

use super::state::{HaltReason, MemoryWrite, PPCEngine, SPR_CTR, SPR_GQR0, SPR_LR, SPR_XER};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegisterSnapshot {
    pub gpr: [u32; 32],
    pub pc: u32,
    pub lr: u32,
    pub ctr: u32,
    pub xer: u32,
    pub cr: u32,
    pub msr: u32,
    pub changed_gpr: Vec<u32>,
    /// GQR0–GQR7 (SPRs 912–919): paired-single quantization config.
    pub gqr: [u32; 8],
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FPUSnapshot {
    pub fpr: Vec<[f64; 2]>,
    pub fpscr: u32,
    pub changed_fpr: Vec<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CallFrame {
    pub call_site: u32,
    pub return_to: u32,
    pub symbol: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MachineStateSnapshot {
    pub registers: RegisterSnapshot,
    pub fpu: FPUSnapshot,
    pub step_count: u64,
    pub halted: bool,
    pub halt_reason: HaltReason,
    pub breakpoints: Vec<u32>,
    pub call_stack: Vec<CallFrame>,
    pub symbols: Vec<(String, u32)>,
    pub program_end: u32,
    pub last_writes: Vec<MemoryWrite>,
}

pub fn to_snapshot(engine: &PPCEngine) -> MachineStateSnapshot {
    MachineStateSnapshot {
        registers: RegisterSnapshot {
            gpr: engine.cpu.gpr,
            pc: engine.cpu.pc,
            lr: engine.cpu.spr[SPR_LR],
            ctr: engine.cpu.spr[SPR_CTR],
            xer: engine.cpu.spr[SPR_XER],
            cr: engine.cpu.cr,
            msr: engine.cpu.msr,
            changed_gpr: engine.changed_gpr.clone(),
            gqr: std::array::from_fn(|i| engine.cpu.spr[SPR_GQR0 + i]),
        },
        fpu: FPUSnapshot {
            fpr: engine.cpu.fpr.to_vec(),
            fpscr: engine.cpu.fpscr,
            changed_fpr: engine.changed_fpr.clone(),
        },
        step_count: engine.step_count,
        halted: engine.halted,
        halt_reason: engine.halt_reason.clone(),
        breakpoints: {
            let mut v: Vec<u32> = engine.breakpoints.iter().copied().collect();
            v.sort_unstable();
            v
        },
        call_stack: engine.call_stack.iter().map(|f| CallFrame {
            call_site: f.call_site,
            return_to: f.return_to,
            symbol: f.symbol.clone(),
        }).collect(),
        symbols: engine.symbols.clone(),
        program_end: engine.program_end,
        last_writes: engine.last_writes.clone(),
    }
}
