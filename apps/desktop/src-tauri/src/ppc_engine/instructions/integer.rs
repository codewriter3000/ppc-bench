//! Integer arithmetic, logical, and compare instructions.
//!
//! Reference: Dolphin `Interpreter_Integer.cpp`.

use super::super::inst::Inst;
use super::super::state::{PPCEngine, XER_CA, XER_OV, XER_SO};
use super::super::tables::Op;
use super::mark_gpr;

#[inline]
fn ra_or_zero(engine: &PPCEngine, inst: Inst) -> u32 {
    if inst.ra() == 0 { 0 } else { engine.cpu.gpr[inst.ra()] }
}

#[inline]
fn ra_val(engine: &PPCEngine, inst: Inst) -> u32 {
    engine.cpu.gpr[inst.ra()]
}

#[inline]
fn write_gpr(engine: &mut PPCEngine, idx: usize, value: u32, update_cr0: bool) {
    engine.cpu.gpr[idx] = value;
    mark_gpr(engine, idx);
    if update_cr0 {
        engine.cpu.update_cr0_signed(value);
    }
}

/// Update XER.CA based on carry from an unsigned add `a + b + carry_in`.
#[inline]
fn set_ca(engine: &mut PPCEngine, a: u32, b: u32, carry_in: u32) {
    let (sum, c1) = a.overflowing_add(b);
    let (_, c2) = sum.overflowing_add(carry_in);
    let ca = c1 || c2;
    if ca {
        engine.cpu.xer |= XER_CA;
    } else {
        engine.cpu.xer &= !XER_CA;
    }
}

/// Update XER.OV/SO for a signed-overflow result.
#[inline]
fn set_ov(engine: &mut PPCEngine, overflow: bool) {
    if overflow {
        engine.cpu.xer |= XER_OV | XER_SO;
    } else {
        engine.cpu.xer &= !XER_OV;
    }
}

pub fn exec_arith(engine: &mut PPCEngine, inst: Inst, op: Op) {
    use Op::*;
    let rd = inst.rd();
    match op {
        Addi => {
            let v = ra_or_zero(engine, inst).wrapping_add(inst.simm() as u32);
            write_gpr(engine, rd, v, false);
        }
        Addis => {
            let v = ra_or_zero(engine, inst).wrapping_add((inst.uimm() << 16) as u32);
            write_gpr(engine, rd, v, false);
        }
        Addic => {
            let a = ra_val(engine, inst);
            let b = inst.simm() as u32;
            set_ca(engine, a, b, 0);
            write_gpr(engine, rd, a.wrapping_add(b), false);
        }
        AddicDot => {
            let a = ra_val(engine, inst);
            let b = inst.simm() as u32;
            set_ca(engine, a, b, 0);
            let v = a.wrapping_add(b);
            write_gpr(engine, rd, v, true);
        }
        Subfic => {
            let a = ra_val(engine, inst);
            let b = inst.simm() as u32;
            // subfic: RT = ~RA + SIMM + 1 (i.e. SIMM - RA)
            set_ca(engine, !a, b, 1);
            write_gpr(engine, rd, b.wrapping_sub(a), false);
        }
        Mulli => {
            let v = (ra_val(engine, inst) as i32).wrapping_mul(inst.simm()) as u32;
            write_gpr(engine, rd, v, false);
        }
        Add | Addo => {
            let a = ra_val(engine, inst);
            let b = engine.cpu.gpr[inst.rb()];
            let (sum, ov) = (a as i32).overflowing_add(b as i32);
            write_gpr(engine, rd, sum as u32, inst.rc());
            if matches!(op, Addo) { set_ov(engine, ov); }
        }
        Addc => {
            let a = ra_val(engine, inst);
            let b = engine.cpu.gpr[inst.rb()];
            set_ca(engine, a, b, 0);
            write_gpr(engine, rd, a.wrapping_add(b), inst.rc());
        }
        Adde => {
            let a = ra_val(engine, inst);
            let b = engine.cpu.gpr[inst.rb()];
            let cin = (engine.cpu.xer & XER_CA) >> 29;
            set_ca(engine, a, b, cin);
            write_gpr(engine, rd, a.wrapping_add(b).wrapping_add(cin), inst.rc());
        }
        Addze => {
            let a = ra_val(engine, inst);
            let cin = (engine.cpu.xer & XER_CA) >> 29;
            set_ca(engine, a, 0, cin);
            write_gpr(engine, rd, a.wrapping_add(cin), inst.rc());
        }
        Addme => {
            let a = ra_val(engine, inst);
            let cin = (engine.cpu.xer & XER_CA) >> 29;
            // addme: RT = RA + CA + (-1)
            set_ca(engine, a, u32::MAX, cin);
            write_gpr(engine, rd, a.wrapping_add(u32::MAX).wrapping_add(cin), inst.rc());
        }
        Subf => {
            // subf: RT = ~RA + RB + 1 = RB - RA
            let a = ra_val(engine, inst);
            let b = engine.cpu.gpr[inst.rb()];
            write_gpr(engine, rd, b.wrapping_sub(a), inst.rc());
        }
        Subfc => {
            let a = ra_val(engine, inst);
            let b = engine.cpu.gpr[inst.rb()];
            set_ca(engine, !a, b, 1);
            write_gpr(engine, rd, b.wrapping_sub(a), inst.rc());
        }
        Subfe => {
            let a = ra_val(engine, inst);
            let b = engine.cpu.gpr[inst.rb()];
            let cin = (engine.cpu.xer & XER_CA) >> 29;
            set_ca(engine, !a, b, cin);
            write_gpr(engine, rd, (!a).wrapping_add(b).wrapping_add(cin), inst.rc());
        }
        Subfze => {
            let a = ra_val(engine, inst);
            let cin = (engine.cpu.xer & XER_CA) >> 29;
            set_ca(engine, !a, 0, cin);
            write_gpr(engine, rd, (!a).wrapping_add(cin), inst.rc());
        }
        Subfme => {
            let a = ra_val(engine, inst);
            let cin = (engine.cpu.xer & XER_CA) >> 29;
            set_ca(engine, !a, u32::MAX, cin);
            write_gpr(engine, rd, (!a).wrapping_add(u32::MAX).wrapping_add(cin), inst.rc());
        }
        Neg => {
            let a = ra_val(engine, inst);
            write_gpr(engine, rd, (!a).wrapping_add(1), inst.rc());
        }
        Mullw => {
            let a = ra_val(engine, inst) as i32;
            let b = engine.cpu.gpr[inst.rb()] as i32;
            write_gpr(engine, rd, a.wrapping_mul(b) as u32, inst.rc());
        }
        Mulhw => {
            let a = ra_val(engine, inst) as i32 as i64;
            let b = engine.cpu.gpr[inst.rb()] as i32 as i64;
            write_gpr(engine, rd, ((a * b) >> 32) as u32, inst.rc());
        }
        Mulhwu => {
            let a = ra_val(engine, inst) as u64;
            let b = engine.cpu.gpr[inst.rb()] as u64;
            write_gpr(engine, rd, ((a * b) >> 32) as u32, inst.rc());
        }
        Divw => {
            let a = ra_val(engine, inst) as i32;
            let b = engine.cpu.gpr[inst.rb()] as i32;
            let v = if b == 0 || (a == i32::MIN && b == -1) {
                if a < 0 { u32::MAX } else { 0 } // undefined; pick deterministic value
            } else {
                (a / b) as u32
            };
            write_gpr(engine, rd, v, inst.rc());
        }
        Divwu => {
            let a = ra_val(engine, inst);
            let b = engine.cpu.gpr[inst.rb()];
            let v = if b == 0 { 0 } else { a / b };
            write_gpr(engine, rd, v, inst.rc());
        }
        _ => unreachable!("exec_arith called with non-arith op {:?}", op),
    }
}

pub fn exec_logical(engine: &mut PPCEngine, inst: Inst, op: Op) {
    use Op::*;
    let rs = inst.rs();
    let ra = inst.ra();
    let s = engine.cpu.gpr[rs];
    let b = engine.cpu.gpr[inst.rb()];
    let v = match op {
        And => s & b,
        Or => s | b,
        Xor => s ^ b,
        Nand => !(s & b),
        Nor => !(s | b),
        Eqv => !(s ^ b),
        Andc => s & !b,
        Orc => s | !b,
        Andi => s & inst.uimm(),
        Andis => s & (inst.uimm() << 16),
        Ori => s | inst.uimm(),
        Oris => s | (inst.uimm() << 16),
        Xori => s ^ inst.uimm(),
        Xoris => s ^ (inst.uimm() << 16),
        Extsb => ((s as i8) as i32) as u32,
        Extsh => ((s as i16) as i32) as u32,
        Cntlzw => s.leading_zeros(),
        Slw => {
            let n = b & 0x3f;
            if n >= 32 { 0 } else { s.wrapping_shl(n) }
        }
        Srw => {
            let n = b & 0x3f;
            if n >= 32 { 0 } else { s.wrapping_shr(n) }
        }
        Sraw => {
            let n = b & 0x3f;
            let sign = (s as i32) < 0;
            let (res, ca) = if n >= 32 {
                (if sign { -1i32 } else { 0 } as u32, sign && s != 0)
            } else {
                let shifted = (s as i32) >> n;
                let mask = (1u32 << n).wrapping_sub(1);
                let ca = sign && (s & mask) != 0;
                (shifted as u32, ca)
            };
            if ca { engine.cpu.xer |= XER_CA; } else { engine.cpu.xer &= !XER_CA; }
            res
        }
        Srawi => {
            let n = inst.sh();
            let sign = (s as i32) < 0;
            let (res, ca) = if n == 0 {
                (s, false)
            } else {
                let shifted = (s as i32) >> n;
                let mask = (1u32 << n).wrapping_sub(1);
                let ca = sign && (s & mask) != 0;
                (shifted as u32, ca)
            };
            if ca { engine.cpu.xer |= XER_CA; } else { engine.cpu.xer &= !XER_CA; }
            res
        }
        Rlwinm => {
            let n = inst.sh();
            let mask = mask_from(inst.mb(), inst.me());
            s.rotate_left(n) & mask
        }
        Rlwnm => {
            let n = b & 0x1f;
            let mask = mask_from(inst.mb(), inst.me());
            s.rotate_left(n) & mask
        }
        Rlwimi => {
            let n = inst.sh();
            let mask = mask_from(inst.mb(), inst.me());
            let cur = engine.cpu.gpr[ra];
            (s.rotate_left(n) & mask) | (cur & !mask)
        }
        _ => unreachable!("exec_logical called with non-logical op {:?}", op),
    };
    // andi./andis. always update CR0; others do so when Rc is set or it's a
    // dot-suffixed op (already encoded in Rc bit).
    let always_update = matches!(op, Andi | Andis);
    write_gpr_alt(engine, ra, v, inst.rc() || always_update);
}

fn write_gpr_alt(engine: &mut PPCEngine, idx: usize, value: u32, update_cr0: bool) {
    engine.cpu.gpr[idx] = value;
    mark_gpr(engine, idx);
    if update_cr0 {
        engine.cpu.update_cr0_signed(value);
    }
}

/// PPC mask: bits MB..=ME (IBM numbering) set; wraps if MB > ME.
fn mask_from(mb: u32, me: u32) -> u32 {
    let mb_le = 31 - mb;
    let me_le = 31 - me;
    if mb <= me {
        let width = mb_le - me_le + 1;
        let m = if width == 32 { u32::MAX } else { (1u32 << width) - 1 };
        m << me_le
    } else {
        // wrap: !(bits ME+1..=MB-1)
        !mask_from(me + 1, mb - 1)
    }
}

pub fn exec_compare(engine: &mut PPCEngine, inst: Inst, op: Op) {
    use Op::*;
    let crfd = inst.crfd();
    let a = engine.cpu.gpr[inst.ra()];
    let (lt, gt, eq) = match op {
        Cmp => {
            let b = engine.cpu.gpr[inst.rb()];
            ((a as i32) < (b as i32), (a as i32) > (b as i32), a == b)
        }
        Cmpi => {
            let b = inst.simm();
            ((a as i32) < b, (a as i32) > b, (a as i32) == b)
        }
        Cmpl => {
            let b = engine.cpu.gpr[inst.rb()];
            (a < b, a > b, a == b)
        }
        Cmpli => {
            let b = inst.uimm();
            (a < b, a > b, a == b)
        }
        _ => unreachable!(),
    };
    let so = (engine.cpu.xer & XER_SO) >> 31;
    let nibble = ((lt as u32) << 3) | ((gt as u32) << 2) | ((eq as u32) << 1) | so;
    engine.cpu.set_cr_field(crfd, nibble);
}
