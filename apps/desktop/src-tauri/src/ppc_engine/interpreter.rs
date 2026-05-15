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

#[cfg(test)]
mod tests {
    use super::step;
    use crate::ppc_engine::memory::BASE_ADDR;
    use crate::ppc_engine::state::{PPCEngine, SPR_SRR0, SPR_SRR1};

    #[test]
    fn rfi_can_resume_from_low_physical_ram_mirror() {
        let mut engine = PPCEngine::new();
        let resume_addr = 0x0000_0100;

        engine.mem.write_u32(BASE_ADDR, 0x4C00_0064).unwrap();
        engine
            .mem
            .write_u32(BASE_ADDR + resume_addr, 0x3860_0001)
            .unwrap();
        engine.program_end = BASE_ADDR + resume_addr + 4;
        engine.cpu.pc = BASE_ADDR;
        engine.cpu.spr[SPR_SRR0] = resume_addr;
        engine.cpu.spr[SPR_SRR1] = 0;

        step(&mut engine).unwrap();
        assert_eq!(engine.cpu.pc, resume_addr);

        step(&mut engine).unwrap();
        assert_eq!(engine.cpu.gpr[3], 1);
    }
}
