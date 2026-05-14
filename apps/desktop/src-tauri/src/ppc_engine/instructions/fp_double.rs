//! Double-precision FP arithmetic + FP move/convert/compare.
//!
//! Reference: Dolphin `Interpreter_FloatingPoint.cpp` (double forms).
//! Only the PS0 slot is written for double-precision ops; PS1 is preserved.

use super::super::inst::Inst;
use super::super::state::PPCEngine;
use super::super::tables::Op;
use super::mark_fpr;

pub fn exec_fp_double(engine: &mut PPCEngine, inst: Inst, op: Op) {
    use Op::*;
    let frd = inst.rd();
    let a = engine.cpu.fpr[inst.ra()][0];
    let b = engine.cpu.fpr[inst.rb()][0];
    let c = engine.cpu.fpr[inst.rc_reg()][0];

    match op {
        Fadd => write_ps0(engine, frd, a + b),
        Fsub => write_ps0(engine, frd, a - b),
        Fmul => write_ps0(engine, frd, a * c),
        Fdiv => write_ps0(engine, frd, a / b),
        Fmadd => write_ps0(engine, frd, (a * c) + b),
        Fmsub => write_ps0(engine, frd, (a * c) - b),
        Fnmadd => write_ps0(engine, frd, -((a * c) + b)),
        Fnmsub => write_ps0(engine, frd, -((a * c) - b)),
        Fsqrt => write_ps0(engine, frd, b.sqrt()),
        Frsp => write_ps0(engine, frd, (b as f32) as f64),
        Fabs => write_ps0(engine, frd, b.abs()),
        Fneg => write_ps0(engine, frd, -b),
        Fnabs => write_ps0(engine, frd, -b.abs()),
        Fmr => write_ps0(engine, frd, b),
        Fsel => write_ps0(engine, frd, if a >= 0.0 { c } else { b }),
        Fctiw | Fctiwz => {
            // Convert to 32-bit integer (truncate for fctiwz; current rounding for fctiw).
            let v = if matches!(op, Fctiwz) { b.trunc() } else { b.round() };
            let i = if v.is_nan() {
                i32::MIN
            } else if v > i32::MAX as f64 {
                i32::MAX
            } else if v < i32::MIN as f64 {
                i32::MIN
            } else {
                v as i32
            };
            let bits = (i as u32) as u64;
            engine.cpu.fpr[frd] = [f64::from_bits(bits | 0xfff8_0000_0000_0000), engine.cpu.fpr[frd][1]];
            mark_fpr(engine, frd);
        }
        Fcmpu | Fcmpo => {
            let (lt, gt, eq, un) = if a.is_nan() || b.is_nan() {
                (false, false, false, true)
            } else {
                (a < b, a > b, a == b, false)
            };
            let nibble = ((lt as u32) << 3) | ((gt as u32) << 2) | ((eq as u32) << 1) | (un as u32);
            engine.cpu.set_cr_field(inst.crfd(), nibble);
        }
        Mffs => {
            // Move FPSCR to FRD (low 32 bits).
            let bits = engine.cpu.fpscr as u64 | 0xfff8_0000_0000_0000;
            engine.cpu.fpr[frd] = [f64::from_bits(bits), engine.cpu.fpr[frd][1]];
            mark_fpr(engine, frd);
        }
        Mtfsf => {
            let bits = engine.cpu.fpr[inst.rb()][0].to_bits() as u32;
            let fm = inst.fm();
            let mut mask: u32 = 0;
            for i in 0..8 {
                if (fm & (1 << (7 - i))) != 0 {
                    mask |= 0xf << (28 - 4 * i);
                }
            }
            engine.cpu.fpscr = (engine.cpu.fpscr & !mask) | (bits & mask);
        }
        _ => unreachable!("exec_fp_double {:?}", op),
    }
}

fn write_ps0(engine: &mut PPCEngine, idx: usize, value: f64) {
    engine.cpu.fpr[idx][0] = value;
    mark_fpr(engine, idx);
}
