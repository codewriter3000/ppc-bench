//! System, SPR, CR, and cache management instructions.
//!
//! Reference: Dolphin `Interpreter_SystemRegisters.cpp` + cache no-op stubs.
//! Cache ops are no-ops in the simulator (we have a flat memory model) but
//! still consume an instruction slot so they show up correctly in the trace.

use super::super::inst::Inst;
use super::super::state::{PPCEngine, SPR_SRR0, SPR_SRR1};
use super::super::tables::Op;
use super::mark_gpr;
use super::StepOutcome;

#[inline]
fn segment_index_from_address(addr: u32) -> usize {
    ((addr >> 28) & 0xf) as usize
}

pub fn exec_rfi(engine: &mut PPCEngine) -> StepOutcome {
    engine.cpu.msr = engine.cpu.spr[SPR_SRR1];
    engine.cpu.npc = engine.cpu.spr[SPR_SRR0] & !0x3;
    StepOutcome::Branched
}

pub fn exec_system(engine: &mut PPCEngine, inst: Inst, op: Op) {
    use Op::*;
    match op {
        Mfcr => {
            let rd = inst.rd();
            engine.cpu.gpr[rd] = engine.cpu.cr;
            mark_gpr(engine, rd);
        }
        Mtcrf => {
            let crm = inst.crm();
            let s = engine.cpu.gpr[inst.rs()];
            let mut mask: u32 = 0;
            for i in 0..8 {
                if (crm & (1 << (7 - i))) != 0 {
                    mask |= 0xf << (28 - 4 * i);
                }
            }
            engine.cpu.cr = (engine.cpu.cr & !mask) | (s & mask);
        }
        Mfspr => {
            let spr = inst.spr() as usize;
            let rd = inst.rd();
            engine.cpu.gpr[rd] = engine.cpu.spr[spr & 0x3ff];
            mark_gpr(engine, rd);
        }
        Mtspr => {
            let spr = inst.spr() as usize;
            engine.cpu.spr[spr & 0x3ff] = engine.cpu.gpr[inst.rs()];
        }
        Mfmsr => {
            let rd = inst.rd();
            engine.cpu.gpr[rd] = engine.cpu.msr;
            mark_gpr(engine, rd);
        }
        Mtmsr => {
            engine.cpu.msr = engine.cpu.gpr[inst.rs()];
        }
        Mfsr => {
            let rd = inst.rd();
            let segment = inst.ra() & 0xf;
            engine.cpu.gpr[rd] = engine.cpu.sr[segment];
            mark_gpr(engine, rd);
        }
        Mtsr => {
            let segment = inst.ra() & 0xf;
            engine.cpu.sr[segment] = engine.cpu.gpr[inst.rs()];
        }
        Mfsrin => {
            let rd = inst.rd();
            let segment = segment_index_from_address(engine.cpu.gpr[inst.rb()]);
            engine.cpu.gpr[rd] = engine.cpu.sr[segment];
            mark_gpr(engine, rd);
        }
        Mtsrin => {
            let segment = segment_index_from_address(engine.cpu.gpr[inst.rb()]);
            engine.cpu.sr[segment] = engine.cpu.gpr[inst.rs()];
        }
        Mcrf => {
            // mcrf crfD, crfS — copy a CR field.
            let s = engine.cpu.cr_field(inst.crfs());
            engine.cpu.set_cr_field(inst.crfd(), s);
        }
        Crand | Cror | Crxor | Crnand | Crnor | Creqv | Crandc | Crorc => {
            let a = engine.cpu.cr_bit(inst.crba()) as u32;
            let b = engine.cpu.cr_bit(inst.crbb()) as u32;
            let r = match op {
                Crand => a & b,
                Cror => a | b,
                Crxor => a ^ b,
                Crnand => 1 ^ (a & b),
                Crnor => 1 ^ (a | b),
                Creqv => 1 ^ (a ^ b),
                Crandc => a & (b ^ 1),
                Crorc => a | (b ^ 1),
                _ => unreachable!(),
            };
            let bit = inst.crbd();
            let shift = 31 - bit;
            engine.cpu.cr = (engine.cpu.cr & !(1 << shift)) | ((r & 1) << shift);
        }
        // Cache + MMU maintenance are no-ops in the flat-memory simulator.
        Sync | Isync | Eieio | Tlbie | Tlbia | Tlbsync | Dcbi | Dcbf | Dcbst | Dcbt | Dcbtst | Icbi => {}
        Dcbz => {
            // dcbz zeroes a 32-byte cache line.
            let base = if inst.ra() == 0 { 0 } else { engine.cpu.gpr[inst.ra()] };
            let ea = base.wrapping_add(engine.cpu.gpr[inst.rb()]) & !0x1f;
            for i in 0..8 {
                let _ = engine.mem.write_u32(ea + i * 4, 0);
            }
        }
        _ => unreachable!("exec_system {:?}", op),
    }
}

#[cfg(test)]
mod tests {
    use super::exec_system;
    use crate::ppc_engine::inst::Inst;
    use crate::ppc_engine::state::PPCEngine;
    use crate::ppc_engine::tables::{decode, Op};

    fn x_form(rd: u32, ra: u32, rb: u32, subop: u32) -> Inst {
        Inst((31 << 26) | (rd << 21) | (ra << 16) | (rb << 11) | (subop << 1))
    }

    #[test]
    fn segment_register_instructions_decode_and_execute() {
        let mut engine = PPCEngine::new();
        engine.cpu.gpr[3] = 0xDEAD_BEEF;

        let mtsr = x_form(3, 0, 0, 210);
        assert_eq!(decode(mtsr), Op::Mtsr);
        exec_system(&mut engine, mtsr, Op::Mtsr);
        assert_eq!(engine.cpu.sr[0], 0xDEAD_BEEF);

        let mfsr = x_form(4, 0, 0, 595);
        assert_eq!(decode(mfsr), Op::Mfsr);
        exec_system(&mut engine, mfsr, Op::Mfsr);
        assert_eq!(engine.cpu.gpr[4], 0xDEAD_BEEF);

        engine.cpu.gpr[5] = 0xCAFE_BABE;
        engine.cpu.gpr[6] = 0xA123_4567;
        let mtsrin = x_form(5, 0, 6, 242);
        assert_eq!(decode(mtsrin), Op::Mtsrin);
        exec_system(&mut engine, mtsrin, Op::Mtsrin);
        assert_eq!(engine.cpu.sr[0xA], 0xCAFE_BABE);

        engine.cpu.gpr[8] = 0xA000_0000;
        let mfsrin = x_form(7, 0, 8, 659);
        assert_eq!(decode(mfsrin), Op::Mfsrin);
        exec_system(&mut engine, mfsrin, Op::Mfsrin);
        assert_eq!(engine.cpu.gpr[7], 0xCAFE_BABE);
    }

    #[test]
    fn tlb_maintenance_instructions_decode_as_noops() {
        let mut engine = PPCEngine::new();
        engine.cpu.gpr[9] = 0x8000_1000;
        engine.cpu.gpr[11] = 0x0000_0001;
        engine.cpu.sr[0] = 0x1234_5678;
        let gpr_before = engine.cpu.gpr;
        let sr_before = engine.cpu.sr;

        let tlbie = x_form(11, 0, 9, 306);
        let tlbia = x_form(0, 0, 0, 370);
        let tlbsync = x_form(0, 0, 0, 566);

        assert_eq!(decode(tlbie), Op::Tlbie);
        assert_eq!(decode(tlbia), Op::Tlbia);
        assert_eq!(decode(tlbsync), Op::Tlbsync);

        exec_system(&mut engine, tlbie, Op::Tlbie);
        exec_system(&mut engine, tlbia, Op::Tlbia);
        exec_system(&mut engine, tlbsync, Op::Tlbsync);

        assert_eq!(engine.cpu.gpr, gpr_before);
        assert_eq!(engine.cpu.sr, sr_before);
    }
}
