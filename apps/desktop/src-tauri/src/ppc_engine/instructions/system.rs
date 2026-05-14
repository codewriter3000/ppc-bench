//! System, SPR, CR, and cache management instructions.
//!
//! Reference: Dolphin `Interpreter_SystemRegisters.cpp` + cache no-op stubs.
//! Cache ops are no-ops in the simulator (we have a flat memory model) but
//! still consume an instruction slot so they show up correctly in the trace.

use super::super::inst::Inst;
use super::super::state::PPCEngine;
use super::super::tables::Op;
use super::mark_gpr;

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
        // Cache + sync — no-ops for the simulator.
        Sync | Isync | Eieio | Dcbi | Dcbf | Dcbst | Dcbt | Dcbtst | Icbi => {}
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
