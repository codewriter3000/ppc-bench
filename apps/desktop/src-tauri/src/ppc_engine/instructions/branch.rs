//! Branch and conditional-branch instructions.
//!
//! Reference: Dolphin `Interpreter_Branch.cpp`. The Gekko branch model:
//!
//!   - BO[0] = "don't decrement CTR" not.   bit 0 = 1 → skip CTR decrement.
//!   - BO[1] = condition-true required.
//!   - BO[2] = "don't test condition" not.  bit 2 = 1 → skip CR test.
//!   - BO[3] = condition value tested for (when condition is tested).
//!   - BO[4] = static branch-prediction hint (ignored here).

use super::super::inst::Inst;
use super::super::state::{HaltReason, PPCEngine, StackFrame, SPR_LR, SPR_CTR};
use super::super::tables::Op;
use super::StepOutcome;

fn branch_taken(engine: &mut PPCEngine, target: u32, lk: bool, inst_pc: u32) -> StepOutcome {
    let return_to = inst_pc.wrapping_add(4);
    if lk {
        engine.cpu.spr[SPR_LR] = return_to;
        engine.call_stack.push(StackFrame {
            call_site: inst_pc,
            return_to,
            symbol: engine
                .symbols
                .iter()
                .find(|(_, a)| *a == target)
                .map(|(n, _)| n.clone()),
        });
    }
    engine.cpu.npc = target;
    StepOutcome::Branched
}

pub fn exec_branch(engine: &mut PPCEngine, inst: Inst, op: Op) -> StepOutcome {
    use Op::*;
    let pc = engine.cpu.pc;
    match op {
        B => {
            let target = if inst.aa() {
                inst.li() as u32
            } else {
                pc.wrapping_add(inst.li() as u32)
            };
            branch_taken(engine, target, inst.lk(), pc)
        }
        Bc => {
            if eval_bc(engine, inst.bo(), inst.bi()) {
                let target = if inst.aa() {
                    inst.bd() as u32
                } else {
                    pc.wrapping_add(inst.bd() as u32)
                };
                branch_taken(engine, target, inst.lk(), pc)
            } else {
                StepOutcome::Next
            }
        }
        Bclr => {
            if eval_bc(engine, inst.bo(), inst.bi()) {
                let target = engine.cpu.spr[SPR_LR] & !0x3;
                // BLR pops a frame from the software call stack (if it matches).
                let popped = if !inst.lk() {
                    if let Some(idx) = engine
                        .call_stack
                        .iter()
                        .rposition(|f| f.return_to == target)
                    {
                        // Truncate to that frame (handles tail-call style nesting).
                        engine.call_stack.truncate(idx);
                        true
                    } else {
                        false
                    }
                } else {
                    false
                };
                let _ = popped;
                if inst.lk() {
                    // bclrl: also overwrite LR.
                    engine.cpu.spr[SPR_LR] = pc.wrapping_add(4);
                }
                engine.cpu.npc = target;
                if target < engine.cpu.pc.min(engine.program_end)
                    && target == 0
                {
                    return StepOutcome::Halt(HaltReason::EndOfProgram);
                }
                StepOutcome::Branched
            } else {
                StepOutcome::Next
            }
        }
        Bcctr => {
            if eval_bc(engine, inst.bo(), inst.bi()) {
                let target = engine.cpu.spr[SPR_CTR] & !0x3;
                if inst.lk() {
                    engine.cpu.spr[SPR_LR] = pc.wrapping_add(4);
                    engine.call_stack.push(StackFrame {
                        call_site: pc,
                        return_to: pc.wrapping_add(4),
                        symbol: None,
                    });
                }
                engine.cpu.npc = target;
                StepOutcome::Branched
            } else {
                StepOutcome::Next
            }
        }
        _ => unreachable!("exec_branch called with {:?}", op),
    }
}

/// Evaluate a BO/BI condition test, including CTR decrement.
/// Returns `true` if the branch should be taken.
fn eval_bc(engine: &mut PPCEngine, bo: u32, bi: u32) -> bool {
    let ctr_dec_skip = (bo & 0b10000) != 0; // BO[0]
    let ctr_zero_req = (bo & 0b01000) != 0; // BO[1]: 1 = require CTR == 0
    let cond_skip = (bo & 0b00100) != 0; // BO[2]
    let cond_value = (bo & 0b00010) != 0; // BO[3]

    let ctr_ok = if ctr_dec_skip {
        true
    } else {
        let new = engine.cpu.spr[SPR_CTR].wrapping_sub(1);
        engine.cpu.spr[SPR_CTR] = new;
        if ctr_zero_req {
            new == 0
        } else {
            new != 0
        }
    };

    let cond_ok = if cond_skip {
        true
    } else {
        engine.cpu.cr_bit(bi) == cond_value
    };

    ctr_ok && cond_ok
}
