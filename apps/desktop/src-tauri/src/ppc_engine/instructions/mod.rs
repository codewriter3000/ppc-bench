//! Instruction implementations.
//!
//! Organized to mirror Dolphin's `Interpreter_*.cpp` files. Each handler
//! receives a mutable reference to the engine and the decoded [`Inst`].
//! Handlers return a [`StepOutcome`] describing branch/halt effects.

pub mod branch;
pub mod fp_double;
pub mod fp_single;
pub mod integer;
pub mod load_store;
pub mod paired;
pub mod system;

use super::inst::Inst;
use super::state::{HaltReason, PPCEngine};
use super::tables::Op;

/// What happened during a single instruction's execution.
#[derive(Debug, Clone)]
pub enum StepOutcome {
    /// Normal advance — PC moves to the next sequential instruction.
    Next,
    /// Branch taken — PC was set explicitly.
    Branched,
    /// Engine should halt (trap, sc, or fatal).
    Halt(HaltReason),
}

/// Top-level dispatcher. Reads [`Op`] from the tables module and forwards.
pub fn dispatch(engine: &mut PPCEngine, inst: Inst, op: Op) -> StepOutcome {
    use Op::*;
    match op {
        // ── Integer ALU ────────────────────────────────────────────────
        Addi | Addis | Addic | AddicDot | Add | Addo | Addc | Adde | Addze | Addme
        | Subf | Subfc | Subfe | Subfic | Subfze | Subfme | Neg
        | Mulli | Mullw | Mulhw | Mulhwu | Divw | Divwu => {
            integer::exec_arith(engine, inst, op);
            StepOutcome::Next
        }
        And | Or | Xor | Nand | Nor | Eqv | Andc | Orc
        | Andi | Andis | Ori | Oris | Xori | Xoris
        | Extsb | Extsh | Cntlzw
        | Slw | Srw | Sraw | Srawi
        | Rlwinm | Rlwnm | Rlwimi => {
            integer::exec_logical(engine, inst, op);
            StepOutcome::Next
        }
        Cmp | Cmpi | Cmpl | Cmpli => {
            integer::exec_compare(engine, inst, op);
            StepOutcome::Next
        }

        // ── Branches ──────────────────────────────────────────────────
        B | Bc | Bclr | Bcctr => branch::exec_branch(engine, inst, op),

        // ── CR / SPR / system ─────────────────────────────────────────
        Mcrf | Crand | Cror | Crxor | Crnand | Crnor | Creqv | Crandc | Crorc
        | Mfcr | Mtcrf | Mfspr | Mtspr | Mfmsr | Mtmsr | Mfsr | Mtsr | Mfsrin | Mtsrin
        | Tlbie | Tlbia | Tlbsync | Sync | Isync | Eieio
        | Dcbz | Dcbi | Dcbf | Dcbst | Dcbt | Dcbtst | Icbi => {
            system::exec_system(engine, inst, op);
            StepOutcome::Next
        }
        Rfi => system::exec_rfi(engine),
        Sc | Tw | Twi => StepOutcome::Halt(HaltReason::Trap),

        // ── Integer load / store ──────────────────────────────────────
        Lbz | Lbzu | Lbzx | Lbzux
        | Lhz | Lhzu | Lhzx | Lhzux | Lha | Lhau | Lhax | Lhaux
        | Lwz | Lwzu | Lwzx | Lwzux
        | Stb | Stbu | Stbx | Stbux
        | Sth | Sthu | Sthx | Sthux
        | Stw | Stwu | Stwx | Stwux
        | Lmw | Stmw | Lwbrx | Stwbrx | Lhbrx | Sthbrx => {
            match load_store::exec_int_ls(engine, inst, op) {
                Ok(()) => StepOutcome::Next,
                Err(e) => StepOutcome::Halt(HaltReason::MemoryError(e.to_string())),
            }
        }

        // ── FP load / store ───────────────────────────────────────────
        Lfs | Lfsu | Lfsx | Lfsux | Lfd | Lfdu | Lfdx | Lfdux
        | Stfs | Stfsu | Stfsx | Stfsux | Stfd | Stfdu | Stfdx | Stfdux => {
            match load_store::exec_fp_ls(engine, inst, op) {
                Ok(()) => StepOutcome::Next,
                Err(e) => StepOutcome::Halt(HaltReason::MemoryError(e.to_string())),
            }
        }

        // ── FP arithmetic ─────────────────────────────────────────────
        Fadds | Fsubs | Fmuls | Fdivs | Fmadds | Fmsubs | Fnmadds | Fnmsubs
        | Fres | Frsqrte => {
            fp_single::exec_fp_single(engine, inst, op);
            StepOutcome::Next
        }
        Fadd | Fsub | Fmul | Fdiv | Fmadd | Fmsub | Fnmadd | Fnmsub | Fsqrt | Frsp
        | Fabs | Fneg | Fnabs | Fmr | Fsel | Fctiw | Fctiwz | Fcmpu | Fcmpo
        | Mtfsf | Mffs => {
            fp_double::exec_fp_double(engine, inst, op);
            StepOutcome::Next
        }

        // ── Paired singles ────────────────────────────────────────────
        PsAdd | PsSub | PsMul | PsDiv | PsMadd | PsMsub | PsNmadd | PsNmsub
        | PsMadds0 | PsMadds1 | PsMuls0 | PsMuls1 | PsSum0 | PsSum1
        | PsMerge00 | PsMerge01 | PsMerge10 | PsMerge11
        | PsAbs | PsNeg | PsNabs | PsMr | PsRes | PsRsqrte | PsSel
        | PsCmpu0 | PsCmpu1 | PsCmpo0 | PsCmpo1
        | PsqL | PsqLu | PsqLx | PsqLux | PsqSt | PsqStu | PsqStx | PsqStux => {
            match paired::exec_paired(engine, inst, op) {
                Ok(()) => StepOutcome::Next,
                Err(e) => StepOutcome::Halt(HaltReason::MemoryError(e.to_string())),
            }
        }

        Unknown => StepOutcome::Halt(HaltReason::InvalidInstruction(inst.raw())),
    }
}

/// Mark `idx` as a changed GPR for the UI delta highlight, if not already
/// recorded this step.
pub(crate) fn mark_gpr(engine: &mut PPCEngine, idx: usize) {
    let v = idx as u32;
    if !engine.changed_gpr.contains(&v) {
        engine.changed_gpr.push(v);
    }
}

pub(crate) fn mark_fpr(engine: &mut PPCEngine, idx: usize) {
    let v = idx as u32;
    if !engine.changed_fpr.contains(&v) {
        engine.changed_fpr.push(v);
    }
}
