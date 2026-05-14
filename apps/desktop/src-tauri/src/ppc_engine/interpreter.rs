//! Fetch/decode/execute loop.

use super::inst::Inst;
use super::instructions::{dispatch, StepOutcome};
use super::state::{HaltReason, PPCEngine, TraceEntry};
use super::tables::{decode, Op};
use super::disassembler::format_operands;

/// Outcome of a single step.
pub struct StepReport {
    pub trace: TraceEntry,
}

/// Execute one instruction. Updates engine state, trace, and call stack.
/// Returns `Err(HaltReason)` if the engine could not continue.
pub fn step(engine: &mut PPCEngine) -> Result<StepReport, HaltReason> {
    if engine.halted {
        return Err(engine.halt_reason.clone());
    }
    engine.changed_gpr.clear();
    engine.changed_fpr.clear();
    engine.last_writes.clear();

    let pc = engine.cpu.pc;

    // End-of-program halt: PC past the last loaded instruction.
    if pc >= engine.program_end {
        engine.halted = true;
        engine.halt_reason = HaltReason::EndOfProgram;
        return Err(HaltReason::EndOfProgram);
    }

    let raw = match engine.mem.read_u32(pc) {
        Ok(v) => v,
        Err(e) => {
            let reason = HaltReason::MemoryError(e.to_string());
            engine.halted = true;
            engine.halt_reason = reason.clone();
            return Err(reason);
        }
    };
    let inst = Inst(raw);
    let op = decode(inst);

    engine.cpu.npc = pc.wrapping_add(4);

    let outcome = dispatch(engine, inst, op);

    let (mnemonic, operands) = render(inst, op);
    let entry = TraceEntry {
        step: engine.step_count,
        pc,
        raw,
        mnemonic: mnemonic.clone(),
        operands: operands.clone(),
    };

    match outcome {
        StepOutcome::Next | StepOutcome::Branched => {
            engine.cpu.pc = engine.cpu.npc;
            engine.step_count += 1;
            engine.push_trace(entry.clone());
            // Breakpoint check after PC advances.
            if engine.breakpoints.contains(&engine.cpu.pc) {
                engine.halted = true;
                engine.halt_reason = HaltReason::Breakpoint(engine.cpu.pc);
            }
            Ok(StepReport { trace: entry })
        }
        StepOutcome::Halt(reason) => {
            engine.push_trace(entry.clone());
            engine.halted = true;
            engine.halt_reason = reason.clone();
            Err(reason)
        }
    }
}

/// Run until breakpoint, halt, or `max_steps` exhausted.
pub fn run_until(engine: &mut PPCEngine, max_steps: u32) -> (u32, HaltReason) {
    for i in 0..max_steps {
        match step(engine) {
            Ok(_) => {
                if engine.halted {
                    return (i + 1, engine.halt_reason.clone());
                }
            }
            Err(r) => return (i + 1, r),
        }
    }
    (max_steps, HaltReason::MaxStepsReached)
}

fn render(inst: Inst, op: Op) -> (String, String) {
    let mnemonic = op.mnemonic().to_string();
    let operands = format_operands(inst, op);
    (mnemonic, operands)
}
