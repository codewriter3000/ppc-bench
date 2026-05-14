//! Integer and FP load/store instructions.
//!
//! Reference: Dolphin `Interpreter_LoadStore.cpp` and `Interpreter_LoadStorePaired.cpp`.

use super::super::inst::Inst;
use super::super::memory::MemError;
use super::super::state::{MemoryWrite, PPCEngine};
use super::super::tables::Op;
use super::{mark_fpr, mark_gpr};

#[inline]
fn ea_d(engine: &PPCEngine, inst: Inst, update: bool) -> u32 {
    let base = if inst.ra() == 0 && !update {
        0
    } else {
        engine.cpu.gpr[inst.ra()]
    };
    base.wrapping_add(inst.simm() as u32)
}

#[inline]
fn ea_x(engine: &PPCEngine, inst: Inst, update: bool) -> u32 {
    let base = if inst.ra() == 0 && !update {
        0
    } else {
        engine.cpu.gpr[inst.ra()]
    };
    base.wrapping_add(engine.cpu.gpr[inst.rb()])
}

fn record_write(engine: &mut PPCEngine, addr: u32, size: u32) {
    engine.last_writes.push(MemoryWrite { addr, size });
}

pub fn exec_int_ls(engine: &mut PPCEngine, inst: Inst, op: Op) -> Result<(), MemError> {
    use Op::*;
    let rd = inst.rd();
    let ra = inst.ra();

    // Compute effective address + decide whether this is an update form.
    let (ea, update) = match op {
        Lbz | Lhz | Lha | Lwz | Stb | Sth | Stw | Lmw | Stmw => (ea_d(engine, inst, false), false),
        Lbzu | Lhzu | Lhau | Lwzu | Stbu | Sthu | Stwu => (ea_d(engine, inst, true), true),
        Lbzx | Lhzx | Lhax | Lwzx | Stbx | Sthx | Stwx | Lwbrx | Stwbrx | Lhbrx | Sthbrx => {
            (ea_x(engine, inst, false), false)
        }
        Lbzux | Lhzux | Lhaux | Lwzux | Stbux | Sthux | Stwux => (ea_x(engine, inst, true), true),
        _ => unreachable!(),
    };

    match op {
        Lbz | Lbzu | Lbzx | Lbzux => {
            let v = engine.mem.read_u8(ea)? as u32;
            engine.cpu.gpr[rd] = v;
            mark_gpr(engine, rd);
        }
        Lhz | Lhzu | Lhzx | Lhzux => {
            let v = engine.mem.read_u16(ea)? as u32;
            engine.cpu.gpr[rd] = v;
            mark_gpr(engine, rd);
        }
        Lha | Lhau | Lhax | Lhaux => {
            let v = (engine.mem.read_u16(ea)? as i16) as i32 as u32;
            engine.cpu.gpr[rd] = v;
            mark_gpr(engine, rd);
        }
        Lwz | Lwzu | Lwzx | Lwzux => {
            let v = engine.mem.read_u32(ea)?;
            engine.cpu.gpr[rd] = v;
            mark_gpr(engine, rd);
        }
        Lwbrx => {
            let v = engine.mem.read_u32(ea)?.swap_bytes();
            engine.cpu.gpr[rd] = v;
            mark_gpr(engine, rd);
        }
        Lhbrx => {
            let v = (engine.mem.read_u16(ea)?.swap_bytes()) as u32;
            engine.cpu.gpr[rd] = v;
            mark_gpr(engine, rd);
        }
        Stb | Stbu | Stbx | Stbux => {
            engine.mem.write_u8(ea, engine.cpu.gpr[inst.rs()] as u8)?;
            record_write(engine, ea, 1);
        }
        Sth | Sthu | Sthx | Sthux => {
            engine.mem.write_u16(ea, engine.cpu.gpr[inst.rs()] as u16)?;
            record_write(engine, ea, 2);
        }
        Stw | Stwu | Stwx | Stwux => {
            engine.mem.write_u32(ea, engine.cpu.gpr[inst.rs()])?;
            record_write(engine, ea, 4);
        }
        Stwbrx => {
            engine.mem.write_u32(ea, engine.cpu.gpr[inst.rs()].swap_bytes())?;
            record_write(engine, ea, 4);
        }
        Sthbrx => {
            let v = (engine.cpu.gpr[inst.rs()] as u16).swap_bytes();
            engine.mem.write_u16(ea, v)?;
            record_write(engine, ea, 2);
        }
        Lmw => {
            let mut addr = ea;
            for i in rd..32 {
                engine.cpu.gpr[i] = engine.mem.read_u32(addr)?;
                mark_gpr(engine, i);
                addr = addr.wrapping_add(4);
            }
        }
        Stmw => {
            let mut addr = ea;
            for i in inst.rs()..32 {
                engine.mem.write_u32(addr, engine.cpu.gpr[i])?;
                record_write(engine, addr, 4);
                addr = addr.wrapping_add(4);
            }
        }
        _ => unreachable!(),
    }

    if update && ra != 0 {
        engine.cpu.gpr[ra] = ea;
        mark_gpr(engine, ra);
    }
    Ok(())
}

pub fn exec_fp_ls(engine: &mut PPCEngine, inst: Inst, op: Op) -> Result<(), MemError> {
    use Op::*;
    let frd = inst.rd();
    let ra = inst.ra();
    let (ea, update) = match op {
        Lfs | Lfd | Stfs | Stfd => (ea_d(engine, inst, false), false),
        Lfsu | Lfdu | Stfsu | Stfdu => (ea_d(engine, inst, true), true),
        Lfsx | Lfdx | Stfsx | Stfdx => (ea_x(engine, inst, false), false),
        Lfsux | Lfdux | Stfsux | Stfdux => (ea_x(engine, inst, true), true),
        _ => unreachable!(),
    };

    match op {
        Lfs | Lfsu | Lfsx | Lfsux => {
            let bits = engine.mem.read_u32(ea)?;
            let v = f32::from_bits(bits) as f64;
            engine.cpu.fpr[frd] = [v, v];
            mark_fpr(engine, frd);
        }
        Lfd | Lfdu | Lfdx | Lfdux => {
            let bits = engine.mem.read_u64(ea)?;
            let v = f64::from_bits(bits);
            engine.cpu.fpr[frd] = [v, engine.cpu.fpr[frd][1]];
            mark_fpr(engine, frd);
        }
        Stfs | Stfsu | Stfsx | Stfsux => {
            let v = engine.cpu.fpr[inst.rs()][0] as f32;
            engine.mem.write_u32(ea, v.to_bits())?;
            record_write(engine, ea, 4);
        }
        Stfd | Stfdu | Stfdx | Stfdux => {
            let bits = engine.cpu.fpr[inst.rs()][0].to_bits();
            engine.mem.write_u64(ea, bits)?;
            record_write(engine, ea, 8);
        }
        _ => unreachable!(),
    }
    if update && ra != 0 {
        engine.cpu.gpr[ra] = ea;
        mark_gpr(engine, ra);
    }
    Ok(())
}
