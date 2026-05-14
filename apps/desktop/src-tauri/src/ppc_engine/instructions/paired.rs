//! Paired-singles instructions (Gekko/Broadway extension).
//!
//! Reference: Dolphin `Interpreter_Paired.cpp` + `Interpreter_LoadStorePaired.cpp`,
//! and the Gekko User's Manual chapter on paired-single FP.
//!
//! Each FPR holds two 32-bit single-precision values: `[ps0, ps1]`. Arithmetic
//! ops execute on both lanes in parallel. Quantized loads/stores (psq_l/st)
//! use the GQR SPRs (GQR0..GQR7) to choose a scaling exponent and a type
//! (single-float / u8 / s8 / u16 / s16). The simulator implements the
//! float scale-factor cases and the simple integer types — that's the common
//! case in Gekko code.

use super::super::inst::Inst;
use super::super::memory::MemError;
use super::super::state::{MemoryWrite, PPCEngine, SPR_GQR0};
use super::super::tables::Op;
use super::{mark_fpr, mark_gpr};

fn to_single(v: f64) -> f64 { (v as f32) as f64 }

fn write_paired(engine: &mut PPCEngine, idx: usize, ps0: f64, ps1: f64) {
    engine.cpu.fpr[idx] = [ps0, ps1];
    mark_fpr(engine, idx);
}

pub fn exec_paired(engine: &mut PPCEngine, inst: Inst, op: Op) -> Result<(), MemError> {
    use Op::*;
    let frd = inst.rd();
    let a = engine.cpu.fpr[inst.ra()];
    let b = engine.cpu.fpr[inst.rb()];
    let c = engine.cpu.fpr[inst.rc_reg()];

    match op {
        PsAdd => write_paired(engine, frd, to_single(a[0] + b[0]), to_single(a[1] + b[1])),
        PsSub => write_paired(engine, frd, to_single(a[0] - b[0]), to_single(a[1] - b[1])),
        PsMul => write_paired(engine, frd, to_single(a[0] * c[0]), to_single(a[1] * c[1])),
        PsDiv => write_paired(engine, frd, to_single(a[0] / b[0]), to_single(a[1] / b[1])),
        PsMadd => write_paired(engine, frd, to_single(a[0] * c[0] + b[0]), to_single(a[1] * c[1] + b[1])),
        PsMsub => write_paired(engine, frd, to_single(a[0] * c[0] - b[0]), to_single(a[1] * c[1] - b[1])),
        PsNmadd => write_paired(engine, frd, to_single(-(a[0] * c[0] + b[0])), to_single(-(a[1] * c[1] + b[1]))),
        PsNmsub => write_paired(engine, frd, to_single(-(a[0] * c[0] - b[0])), to_single(-(a[1] * c[1] - b[1]))),

        // ps_madds0: scale C using ps0 only on both lanes.
        PsMadds0 => write_paired(engine, frd, to_single(a[0] * c[0] + b[0]), to_single(a[1] * c[0] + b[1])),
        PsMadds1 => write_paired(engine, frd, to_single(a[0] * c[1] + b[0]), to_single(a[1] * c[1] + b[1])),
        PsMuls0 => write_paired(engine, frd, to_single(a[0] * c[0]), to_single(a[1] * c[0])),
        PsMuls1 => write_paired(engine, frd, to_single(a[0] * c[1]), to_single(a[1] * c[1])),
        PsSum0 => write_paired(engine, frd, to_single(a[0] + b[1]), to_single(c[1])),
        PsSum1 => write_paired(engine, frd, to_single(c[0]), to_single(a[0] + b[1])),

        PsMerge00 => write_paired(engine, frd, a[0], b[0]),
        PsMerge01 => write_paired(engine, frd, a[0], b[1]),
        PsMerge10 => write_paired(engine, frd, a[1], b[0]),
        PsMerge11 => write_paired(engine, frd, a[1], b[1]),

        PsAbs => write_paired(engine, frd, b[0].abs(), b[1].abs()),
        PsNeg => write_paired(engine, frd, -b[0], -b[1]),
        PsNabs => write_paired(engine, frd, -b[0].abs(), -b[1].abs()),
        PsMr => write_paired(engine, frd, b[0], b[1]),
        PsRes => write_paired(engine, frd, to_single(1.0 / b[0]), to_single(1.0 / b[1])),
        PsRsqrte => write_paired(engine, frd, to_single(1.0 / b[0].sqrt()), to_single(1.0 / b[1].sqrt())),
        PsSel => write_paired(
            engine, frd,
            if a[0] >= 0.0 { c[0] } else { b[0] },
            if a[1] >= 0.0 { c[1] } else { b[1] },
        ),

        PsCmpu0 | PsCmpo0 => compare_lane(engine, inst.crfd(), a[0], b[0]),
        PsCmpu1 | PsCmpo1 => compare_lane(engine, inst.crfd(), a[1], b[1]),

        // ── Quantized load / store ────────────────────────────────────
        PsqL | PsqLu => exec_psq_load(engine, inst, /*indexed*/ false, /*update*/ matches!(op, PsqLu))?,
        PsqLx | PsqLux => exec_psq_load(engine, inst, true, matches!(op, PsqLux))?,
        PsqSt | PsqStu => exec_psq_store(engine, inst, false, matches!(op, PsqStu))?,
        PsqStx | PsqStux => exec_psq_store(engine, inst, true, matches!(op, PsqStux))?,

        _ => unreachable!("exec_paired {:?}", op),
    }
    Ok(())
}

fn compare_lane(engine: &mut PPCEngine, crfd: u32, x: f64, y: f64) {
    let (lt, gt, eq, un) = if x.is_nan() || y.is_nan() {
        (false, false, false, true)
    } else {
        (x < y, x > y, x == y, false)
    };
    let nibble = ((lt as u32) << 3) | ((gt as u32) << 2) | ((eq as u32) << 1) | (un as u32);
    engine.cpu.set_cr_field(crfd, nibble);
}

/// Decode a GQR entry into (load_type, scale_exponent_signed).
fn gqr_load_params(engine: &PPCEngine, i: u32) -> (u32, i32) {
    let g = engine.cpu.spr[SPR_GQR0 + (i as usize & 7)];
    let ltype = (g >> 16) & 0x7;
    // 6-bit sign-extended scale at bits 24..29 (IBM 8..13).
    let scale = ((g >> 8) & 0x3f) as i32;
    let scale = (scale << 26) >> 26;
    (ltype, scale)
}

fn gqr_store_params(engine: &PPCEngine, i: u32) -> (u32, i32) {
    let g = engine.cpu.spr[SPR_GQR0 + (i as usize & 7)];
    let stype = g & 0x7;
    let scale = ((g >> 24) & 0x3f) as i32;
    let scale = (scale << 26) >> 26;
    (stype, scale)
}

fn dequantize(raw: u32, ltype: u32, scale: i32, halfword: bool) -> f64 {
    let v = if halfword { raw & 0xffff } else { raw & 0xff };
    let signed = match ltype {
        0 | 4 => return f32::from_bits(raw).into(), // float
        5 => v as i8 as f64,                          // signed byte
        6 => v as i16 as f64,                         // signed half
        1 => v as u8 as f64,                          // unsigned byte
        2 => v as u16 as f64,                         // unsigned half
        _ => v as f64,
    };
    let factor = 2f64.powi(-scale);
    signed * factor
}

fn quantize(value: f64, stype: u32, scale: i32) -> u64 {
    match stype {
        0 | 4 => (value as f32).to_bits() as u64,
        5 => {
            let v = (value * 2f64.powi(scale)).clamp(-128.0, 127.0) as i8;
            (v as u8) as u64
        }
        6 => {
            let v = (value * 2f64.powi(scale)).clamp(-32768.0, 32767.0) as i16;
            (v as u16) as u64
        }
        1 => {
            let v = (value * 2f64.powi(scale)).clamp(0.0, 255.0) as u8;
            v as u64
        }
        2 => {
            let v = (value * 2f64.powi(scale)).clamp(0.0, 65535.0) as u16;
            v as u64
        }
        _ => (value as f32).to_bits() as u64,
    }
}

fn element_size(t: u32) -> u32 {
    match t {
        0 | 4 => 4, // float
        1 | 5 => 1, // byte
        2 | 6 => 2, // half
        _ => 4,
    }
}

fn exec_psq_load(
    engine: &mut PPCEngine,
    inst: Inst,
    indexed: bool,
    update: bool,
) -> Result<(), MemError> {
    let frd = inst.rd();
    let ra = inst.ra();
    let base = if ra == 0 && !update { 0 } else { engine.cpu.gpr[ra] };
    let ea = if indexed {
        base.wrapping_add(engine.cpu.gpr[inst.rb()])
    } else {
        base.wrapping_add(inst.psq_d() as u32)
    };
    let (ltype, scale) = gqr_load_params(engine, inst.i());
    let single = inst.w();
    let size = element_size(ltype);

    let ps0_raw = read_sized(engine, ea, size)?;
    let halfword = size == 2;
    let ps0 = dequantize(ps0_raw, ltype, scale, halfword);
    let ps1 = if single {
        1.0
    } else {
        let ps1_raw = read_sized(engine, ea.wrapping_add(size), size)?;
        dequantize(ps1_raw, ltype, scale, halfword)
    };
    write_paired(engine, frd, ps0, ps1);
    if update && ra != 0 {
        engine.cpu.gpr[ra] = ea;
        mark_gpr(engine, ra);
    }
    Ok(())
}

fn exec_psq_store(
    engine: &mut PPCEngine,
    inst: Inst,
    indexed: bool,
    update: bool,
) -> Result<(), MemError> {
    let frs = inst.rs();
    let ra = inst.ra();
    let base = if ra == 0 && !update { 0 } else { engine.cpu.gpr[ra] };
    let ea = if indexed {
        base.wrapping_add(engine.cpu.gpr[inst.rb()])
    } else {
        base.wrapping_add(inst.psq_d() as u32)
    };
    let (stype, scale) = gqr_store_params(engine, inst.i());
    let single = inst.w();
    let size = element_size(stype);

    let v0 = quantize(engine.cpu.fpr[frs][0], stype, scale);
    write_sized(engine, ea, v0, size)?;
    if !single {
        let v1 = quantize(engine.cpu.fpr[frs][1], stype, scale);
        write_sized(engine, ea.wrapping_add(size), v1, size)?;
    }
    engine.last_writes.push(MemoryWrite {
        addr: ea,
        size: if single { size } else { size * 2 },
    });
    if update && ra != 0 {
        engine.cpu.gpr[ra] = ea;
        mark_gpr(engine, ra);
    }
    Ok(())
}

fn read_sized(engine: &PPCEngine, addr: u32, size: u32) -> Result<u32, MemError> {
    Ok(match size {
        1 => engine.mem.read_u8(addr)? as u32,
        2 => engine.mem.read_u16(addr)? as u32,
        _ => engine.mem.read_u32(addr)?,
    })
}

fn write_sized(engine: &mut PPCEngine, addr: u32, value: u64, size: u32) -> Result<(), MemError> {
    match size {
        1 => engine.mem.write_u8(addr, value as u8),
        2 => engine.mem.write_u16(addr, value as u16),
        _ => engine.mem.write_u32(addr, value as u32),
    }
}
