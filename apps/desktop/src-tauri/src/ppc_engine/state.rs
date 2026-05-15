//! PPC machine state + engine wrapper.

use std::collections::{HashSet, VecDeque};

use serde::{Deserialize, Serialize};
use thiserror::Error;

use super::memory::{MemError, Memory, BASE_ADDR};

/// SPR numbers (subset; matches Dolphin's `Gekko.h` SPR_* defines).
pub const SPR_XER: usize = 1;
pub const SPR_LR: usize = 8;
pub const SPR_CTR: usize = 9;
pub const SPR_SRR0: usize = 26;
pub const SPR_SRR1: usize = 27;
pub const SPR_GQR0: usize = 912;
pub const SPR_DEC: usize = 22;
pub const SPR_TBL: usize = 268;
pub const SPR_TBU: usize = 269;

/// XER bit masks.
pub const XER_CA: u32 = 1 << 29;
pub const XER_OV: u32 = 1 << 30;
pub const XER_SO: u32 = 1 << 31;

/// Trace ring buffer capacity (matches plan).
pub const TRACE_CAP: usize = 1000;

/// Per-step trace entry.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TraceEntry {
    pub step: u64,
    pub pc: u32,
    pub raw: u32,
    pub mnemonic: String,
    pub operands: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StackFrame {
    pub call_site: u32,
    pub return_to: u32,
    pub symbol: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryWrite {
    pub addr: u32,
    pub size: u32,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum WatchpointKind {
    Write,
    Read,
    Access,
}

#[derive(Debug, Clone)]
pub enum LaunchImage {
    SyntheticProgram { bytes: Vec<u8>, load_addr: u32 },
    OriginalBinary { bytes: Vec<u8>, extension: String },
}

/// Why the engine stopped a `step`/`run_until` call.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum HaltReason {
    Running,
    Breakpoint(u32),
    Watchpoint { kind: WatchpointKind, address: u32 },
    Signal { signal: u8, exception_code: Option<String> },
    EndOfProgram,
    Trap,
    InvalidInstruction(u32),
    MemoryError(String),
    MaxStepsReached,
}

#[derive(Debug, Error)]
pub enum EngineError {
    #[error("memory error: {0}")]
    Memory(#[from] MemError),
    #[error("invalid instruction 0x{0:08X} at 0x{1:08X}")]
    InvalidInstruction(u32, u32),
    #[error("engine halted")]
    Halted,
}

/// Full PowerPC state — mirrors Dolphin's `PowerPCState`.
#[derive(Debug)]
pub struct PowerPCState {
    pub gpr: [u32; 32],
    /// Paired-singles FPRs: `fpr[i] = [ps0, ps1]`.
    pub fpr: [[f64; 2]; 32],
    /// Segment registers SR0-SR15.
    pub sr: [u32; 16],
    pub pc: u32,
    pub npc: u32,
    pub spr: Vec<u32>, // length 1024; Vec to keep stack small
    pub cr: u32,
    pub xer: u32,
    pub msr: u32,
    pub fpscr: u32,
}

impl PowerPCState {
    pub fn new() -> Self {
        Self {
            gpr: [0; 32],
            fpr: [[0.0; 2]; 32],
            sr: [0; 16],
            pc: BASE_ADDR,
            npc: BASE_ADDR,
            spr: vec![0; 1024],
            cr: 0,
            xer: 0,
            msr: 0,
            fpscr: 0,
        }
    }

    #[inline] pub fn lr(&self) -> u32 { self.spr[SPR_LR] }
    #[inline] pub fn ctr(&self) -> u32 { self.spr[SPR_CTR] }
    #[inline] pub fn set_lr(&mut self, v: u32) { self.spr[SPR_LR] = v; }
    #[inline] pub fn set_ctr(&mut self, v: u32) { self.spr[SPR_CTR] = v; }

    /// Update CR field `n` (0..8) with the four nybble bits packed as
    /// `(LT << 3) | (GT << 2) | (EQ << 1) | SO`. PPC stores CR0 in the
    /// high nibble (bits 0..4 IBM = MSB), so field N occupies bits
    /// `(28 - 4*N)..(31 - 4*N)` in little-endian shifts.
    #[inline]
    pub fn set_cr_field(&mut self, n: u32, nibble: u32) {
        let shift = 28 - 4 * n;
        self.cr = (self.cr & !(0xf << shift)) | ((nibble & 0xf) << shift);
    }
    #[inline]
    pub fn cr_field(&self, n: u32) -> u32 {
        let shift = 28 - 4 * n;
        (self.cr >> shift) & 0xf
    }
    /// Test CR bit `idx` (0..32, IBM numbering where 0 is the MSB).
    #[inline]
    pub fn cr_bit(&self, idx: u32) -> bool {
        ((self.cr >> (31 - idx)) & 1) != 0
    }

    /// Common CR0 update: LT/GT/EQ from a signed comparison of `result` to 0,
    /// plus SO copied from XER.
    pub fn update_cr0_signed(&mut self, result: u32) {
        let r = result as i32;
        let lt = (r < 0) as u32;
        let gt = (r > 0) as u32;
        let eq = (r == 0) as u32;
        let so = (self.xer & XER_SO) >> 31;
        let nibble = (lt << 3) | (gt << 2) | (eq << 1) | so;
        self.set_cr_field(0, nibble);
    }
}

impl Default for PowerPCState {
    fn default() -> Self { Self::new() }
}

/// Public engine handle. Owns state, memory, breakpoints, trace, symbols.
pub struct PPCEngine {
    pub cpu: PowerPCState,
    pub mem: Memory,
    pub breakpoints: HashSet<u32>,
    pub trace: VecDeque<TraceEntry>,
    pub call_stack: Vec<StackFrame>,
    pub step_count: u64,
    pub halted: bool,
    pub halt_reason: HaltReason,
    /// Symbols from the most recent assemble.
    pub symbols: Vec<(String, u32)>,
    /// Address one past the last loaded instruction (used as soft end-of-program).
    pub program_end: u32,
    /// Memory addresses written by the most recent step (cleared each step).
    pub last_writes: Vec<MemoryWrite>,
    /// GPR / FPR indices that changed during the most recent step.
    pub changed_gpr: Vec<u32>,
    pub changed_fpr: Vec<u32>,
    pub launch_image: Option<LaunchImage>,
}

impl PPCEngine {
    pub fn new() -> Self {
        Self {
            cpu: PowerPCState::new(),
            mem: Memory::new(),
            breakpoints: HashSet::new(),
            trace: VecDeque::with_capacity(TRACE_CAP),
            call_stack: Vec::new(),
            step_count: 0,
            halted: false,
            halt_reason: HaltReason::Running,
            symbols: Vec::new(),
            program_end: BASE_ADDR,
            last_writes: Vec::new(),
            changed_gpr: Vec::new(),
            changed_fpr: Vec::new(),
            launch_image: None,
        }
    }

    /// Full reset: zero registers/memory/trace; PC back to base.
    pub fn reset(&mut self) {
        let symbols = std::mem::take(&mut self.symbols);
        let program_end = self.program_end;
        let breakpoints = std::mem::take(&mut self.breakpoints);
        let launch_image = self.launch_image.take();
        self.cpu = PowerPCState::new();
        self.mem.clear();
        self.trace.clear();
        self.call_stack.clear();
        self.step_count = 0;
        self.halted = false;
        self.halt_reason = HaltReason::Running;
        self.last_writes.clear();
        self.changed_gpr.clear();
        self.changed_fpr.clear();
        // Preserve user-set breakpoints + symbols across a soft reset.
        self.breakpoints = breakpoints;
        self.symbols = symbols;
        self.program_end = program_end;
        self.launch_image = launch_image;
    }

    pub fn push_trace(&mut self, entry: TraceEntry) {
        if self.trace.len() >= TRACE_CAP {
            self.trace.pop_front();
        }
        self.trace.push_back(entry);
    }
}

impl Default for PPCEngine {
    fn default() -> Self { Self::new() }
}
