//! Single-precision FP arithmetic.
//!
//! Reference: Dolphin `Interpreter_FloatingPoint.cpp` (single forms).
//! Operations are performed in `f64` then rounded to single precision
//! (the same approach Dolphin uses) and broadcast to both paired slots
//! to match Gekko's "PS0 = PS1 = result" semantics for `fxxxs`.

use super::super::inst::Inst;
use super::super::state::PPCEngine;
use super::super::tables::Op;
use super::mark_fpr;

#[inline]
fn to_single(v: f64) -> f64 { (v as f32) as f64 }

pub fn exec_fp_single(engine: &mut PPCEngine, inst: Inst, op: Op) {
    use Op::*;
    let frd = inst.rd();
    let a = engine.cpu.fpr[inst.ra()][0];
    let b = engine.cpu.fpr[inst.rb()][0];
    let c = engine.cpu.fpr[inst.rc_reg()][0];
    let v = match op {
        Fadds => to_single(a + b),
        Fsubs => to_single(a - b),
        Fmuls => to_single(a * c),
        Fdivs => to_single(a / b),
        Fmadds => to_single((a * c) + b),
        Fmsubs => to_single((a * c) - b),
        Fnmadds => to_single(-((a * c) + b)),
        Fnmsubs => to_single(-((a * c) - b)),
        Fres => to_single(1.0 / b),
        Frsqrte => to_single(1.0 / b.sqrt()),
        _ => unreachable!(),
    };
    engine.cpu.fpr[frd] = [v, v];
    mark_fpr(engine, frd);
}
