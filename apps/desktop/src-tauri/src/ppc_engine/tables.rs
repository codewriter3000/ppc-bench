//! Opcode metadata tables — ported in spirit from Dolphin's `PPCTables.cpp`.
//!
//! For disassembly we map `(opcd, subop)` → mnemonic. The interpreter uses
//! a parallel dispatch over the same identifiers. Coverage spans the
//! Gekko/Broadway integer, branch, load/store, FP single/double, and
//! paired-single instructions documented in the IBM PowerPC UISA + Gekko UM.

use super::inst::Inst;

/// Identifier for a decoded instruction. The interpreter dispatches on this
/// enum; the disassembler turns it into a mnemonic string.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
pub enum Op {
    // ── Integer arithmetic ─────────────────────────────────────────────
    Addi, Addis, Addic, AddicDot, Add, Addo, Addc, Adde, Addze, Addme,
    Subf, Subfc, Subfe, Subfic, Subfze, Subfme, Neg,
    Mulli, Mullw, Mulhw, Mulhwu,
    Divw, Divwu,

    // ── Logical / shift / rotate ───────────────────────────────────────
    And, Or, Xor, Nand, Nor, Eqv, Andc, Orc, Andi, Andis, Ori, Oris, Xori, Xoris,
    Extsb, Extsh, Cntlzw,
    Slw, Srw, Sraw, Srawi,
    Rlwinm, Rlwnm, Rlwimi,

    // ── Compare ────────────────────────────────────────────────────────
    Cmp, Cmpi, Cmpl, Cmpli,

    // ── Branch ────────────────────────────────────────────────────────
    B, Bc, Bclr, Bcctr,

    // ── CR ops ─────────────────────────────────────────────────────────
    Mcrf, Crand, Cror, Crxor, Crnand, Crnor, Creqv, Crandc, Crorc,
    Mfcr, Mtcrf,

    // ── System / SPR ───────────────────────────────────────────────────
    Mfspr, Mtspr, Mfmsr, Mtmsr, Sync, Isync, Eieio, Sc, Twi, Tw,

    // ── Cache (no-ops for the simulator, decoded for disasm) ──────────
    Dcbz, Dcbi, Dcbf, Dcbst, Dcbt, Dcbtst, Icbi,

    // ── Integer load / store ──────────────────────────────────────────
    Lbz, Lbzu, Lbzx, Lbzux,
    Lhz, Lhzu, Lhzx, Lhzux, Lha, Lhau, Lhax, Lhaux,
    Lwz, Lwzu, Lwzx, Lwzux,
    Stb, Stbu, Stbx, Stbux,
    Sth, Sthu, Sthx, Sthux,
    Stw, Stwu, Stwx, Stwux,
    Lmw, Stmw,
    Lwbrx, Stwbrx, Lhbrx, Sthbrx,

    // ── FP load / store ───────────────────────────────────────────────
    Lfs, Lfsu, Lfsx, Lfsux,
    Lfd, Lfdu, Lfdx, Lfdux,
    Stfs, Stfsu, Stfsx, Stfsux,
    Stfd, Stfdu, Stfdx, Stfdux,

    // ── FP arithmetic — single ────────────────────────────────────────
    Fadds, Fsubs, Fmuls, Fdivs, Fmadds, Fmsubs, Fnmadds, Fnmsubs, Fres, Frsqrte,

    // ── FP arithmetic — double ────────────────────────────────────────
    Fadd, Fsub, Fmul, Fdiv, Fmadd, Fmsub, Fnmadd, Fnmsub, Fsqrt, Frsp,
    Fabs, Fneg, Fnabs, Fmr, Fsel,
    Fctiw, Fctiwz, Fcmpu, Fcmpo, Mtfsf, Mffs,

    // ── Paired singles (Gekko) ────────────────────────────────────────
    PsAdd, PsSub, PsMul, PsDiv, PsMadd, PsMsub, PsNmadd, PsNmsub,
    PsMadds0, PsMadds1, PsMuls0, PsMuls1, PsSum0, PsSum1,
    PsMerge00, PsMerge01, PsMerge10, PsMerge11,
    PsAbs, PsNeg, PsNabs, PsMr, PsRes, PsRsqrte, PsSel, PsCmpu0, PsCmpu1, PsCmpo0, PsCmpo1,
    PsqL, PsqLu, PsqLx, PsqLux,
    PsqSt, PsqStu, PsqStx, PsqStux,

    /// Unknown / unimplemented — disassembled as `.word 0x...`.
    Unknown,
}

impl Op {
    /// Canonical mnemonic for disassembly. Does not include the `.` (Rc) or
    /// `o` (OE) suffix; the disassembler appends those at print time.
    pub fn mnemonic(self) -> &'static str {
        use Op::*;
        match self {
            Addi => "addi", Addis => "addis", Addic => "addic", AddicDot => "addic.",
            Add => "add", Addo => "addo", Addc => "addc", Adde => "adde",
            Addze => "addze", Addme => "addme",
            Subf => "subf", Subfc => "subfc", Subfe => "subfe", Subfic => "subfic",
            Subfze => "subfze", Subfme => "subfme", Neg => "neg",
            Mulli => "mulli", Mullw => "mullw", Mulhw => "mulhw", Mulhwu => "mulhwu",
            Divw => "divw", Divwu => "divwu",

            And => "and", Or => "or", Xor => "xor", Nand => "nand", Nor => "nor",
            Eqv => "eqv", Andc => "andc", Orc => "orc",
            Andi => "andi.", Andis => "andis.", Ori => "ori", Oris => "oris",
            Xori => "xori", Xoris => "xoris",
            Extsb => "extsb", Extsh => "extsh", Cntlzw => "cntlzw",
            Slw => "slw", Srw => "srw", Sraw => "sraw", Srawi => "srawi",
            Rlwinm => "rlwinm", Rlwnm => "rlwnm", Rlwimi => "rlwimi",

            Cmp => "cmp", Cmpi => "cmpi", Cmpl => "cmpl", Cmpli => "cmpli",

            B => "b", Bc => "bc", Bclr => "bclr", Bcctr => "bcctr",

            Mcrf => "mcrf", Crand => "crand", Cror => "cror", Crxor => "crxor",
            Crnand => "crnand", Crnor => "crnor", Creqv => "creqv",
            Crandc => "crandc", Crorc => "crorc",
            Mfcr => "mfcr", Mtcrf => "mtcrf",

            Mfspr => "mfspr", Mtspr => "mtspr", Mfmsr => "mfmsr", Mtmsr => "mtmsr",
            Sync => "sync", Isync => "isync", Eieio => "eieio", Sc => "sc",
            Twi => "twi", Tw => "tw",

            Dcbz => "dcbz", Dcbi => "dcbi", Dcbf => "dcbf", Dcbst => "dcbst",
            Dcbt => "dcbt", Dcbtst => "dcbtst", Icbi => "icbi",

            Lbz => "lbz", Lbzu => "lbzu", Lbzx => "lbzx", Lbzux => "lbzux",
            Lhz => "lhz", Lhzu => "lhzu", Lhzx => "lhzx", Lhzux => "lhzux",
            Lha => "lha", Lhau => "lhau", Lhax => "lhax", Lhaux => "lhaux",
            Lwz => "lwz", Lwzu => "lwzu", Lwzx => "lwzx", Lwzux => "lwzux",
            Stb => "stb", Stbu => "stbu", Stbx => "stbx", Stbux => "stbux",
            Sth => "sth", Sthu => "sthu", Sthx => "sthx", Sthux => "sthux",
            Stw => "stw", Stwu => "stwu", Stwx => "stwx", Stwux => "stwux",
            Lmw => "lmw", Stmw => "stmw",
            Lwbrx => "lwbrx", Stwbrx => "stwbrx", Lhbrx => "lhbrx", Sthbrx => "sthbrx",

            Lfs => "lfs", Lfsu => "lfsu", Lfsx => "lfsx", Lfsux => "lfsux",
            Lfd => "lfd", Lfdu => "lfdu", Lfdx => "lfdx", Lfdux => "lfdux",
            Stfs => "stfs", Stfsu => "stfsu", Stfsx => "stfsx", Stfsux => "stfsux",
            Stfd => "stfd", Stfdu => "stfdu", Stfdx => "stfdx", Stfdux => "stfdux",

            Fadds => "fadds", Fsubs => "fsubs", Fmuls => "fmuls", Fdivs => "fdivs",
            Fmadds => "fmadds", Fmsubs => "fmsubs", Fnmadds => "fnmadds", Fnmsubs => "fnmsubs",
            Fres => "fres", Frsqrte => "frsqrte",

            Fadd => "fadd", Fsub => "fsub", Fmul => "fmul", Fdiv => "fdiv",
            Fmadd => "fmadd", Fmsub => "fmsub", Fnmadd => "fnmadd", Fnmsub => "fnmsub",
            Fsqrt => "fsqrt", Frsp => "frsp",
            Fabs => "fabs", Fneg => "fneg", Fnabs => "fnabs", Fmr => "fmr", Fsel => "fsel",
            Fctiw => "fctiw", Fctiwz => "fctiwz", Fcmpu => "fcmpu", Fcmpo => "fcmpo",
            Mtfsf => "mtfsf", Mffs => "mffs",

            PsAdd => "ps_add", PsSub => "ps_sub", PsMul => "ps_mul", PsDiv => "ps_div",
            PsMadd => "ps_madd", PsMsub => "ps_msub", PsNmadd => "ps_nmadd", PsNmsub => "ps_nmsub",
            PsMadds0 => "ps_madds0", PsMadds1 => "ps_madds1",
            PsMuls0 => "ps_muls0", PsMuls1 => "ps_muls1",
            PsSum0 => "ps_sum0", PsSum1 => "ps_sum1",
            PsMerge00 => "ps_merge00", PsMerge01 => "ps_merge01",
            PsMerge10 => "ps_merge10", PsMerge11 => "ps_merge11",
            PsAbs => "ps_abs", PsNeg => "ps_neg", PsNabs => "ps_nabs", PsMr => "ps_mr",
            PsRes => "ps_res", PsRsqrte => "ps_rsqrte", PsSel => "ps_sel",
            PsCmpu0 => "ps_cmpu0", PsCmpu1 => "ps_cmpu1",
            PsCmpo0 => "ps_cmpo0", PsCmpo1 => "ps_cmpo1",
            PsqL => "psq_l", PsqLu => "psq_lu", PsqLx => "psq_lx", PsqLux => "psq_lux",
            PsqSt => "psq_st", PsqStu => "psq_stu", PsqStx => "psq_stx", PsqStux => "psq_stux",

            Unknown => ".word",
        }
    }
}

/// Decode a 32-bit instruction word into an [`Op`] identifier.
///
/// Tables are walked in the same order as Dolphin's `s_primary_table` →
/// `s_table4/19/31/59/63` hierarchy.
pub fn decode(inst: Inst) -> Op {
    use Op::*;
    let i = inst.0;
    match inst.opcd() {
        // Primary opcodes (D-form mostly).
        2 => Tw, // tdi/twi mismatch; using Tw — twi is opcd 3
        3 => Twi,
        4 => decode_table_4(inst),
        7 => Mulli,
        8 => Subfic,
        10 => Cmpli,
        11 => Cmpi,
        12 => Addic,
        13 => AddicDot,
        14 => Addi,
        15 => Addis,
        16 => Bc,
        17 => Sc,
        18 => B,
        19 => decode_table_19(inst),
        20 => Rlwimi,
        21 => Rlwinm,
        23 => Rlwnm,
        24 => Ori,
        25 => Oris,
        26 => Xori,
        27 => Xoris,
        28 => Andi,
        29 => Andis,
        31 => decode_table_31(inst),
        32 => Lwz, 33 => Lwzu, 34 => Lbz, 35 => Lbzu,
        36 => Stw, 37 => Stwu, 38 => Stb, 39 => Stbu,
        40 => Lhz, 41 => Lhzu, 42 => Lha, 43 => Lhau,
        44 => Sth, 45 => Sthu,
        46 => Lmw, 47 => Stmw,
        48 => Lfs, 49 => Lfsu, 50 => Lfd, 51 => Lfdu,
        52 => Stfs, 53 => Stfsu, 54 => Stfd, 55 => Stfdu,
        56 => PsqL, 57 => PsqLu,
        59 => decode_table_59(inst),
        60 => PsqSt, 61 => PsqStu,
        63 => decode_table_63(inst),
        _ => {
            let _ = i;
            Unknown
        }
    }
}

fn decode_table_4(inst: Inst) -> Op {
    use Op::*;
    // Paired singles + a couple of A-form FP variants live under opcd=4.
    // Distinguish A-form (5-bit subop) from X-form (10-bit subop).
    match inst.subop5() {
        18 => return PsDiv,
        20 => return PsSub,
        21 => return PsAdd,
        23 => return PsSel,
        24 => return PsRes,
        25 => return PsMul,
        26 => return PsRsqrte,
        28 => return PsMsub,
        29 => return PsMadd,
        30 => return PsNmsub,
        31 => return PsNmadd,
        12 => return PsMuls0,
        13 => return PsMuls1,
        14 => return PsMadds0,
        15 => return PsMadds1,
        10 => return PsSum0,
        11 => return PsSum1,
        _ => {}
    }
    match inst.subop10() {
        0 => PsCmpu0,
        32 => PsCmpo0,
        64 => PsCmpu1,
        96 => PsCmpo1,
        40 => PsNeg,
        72 => PsMr,
        136 => PsNabs,
        264 => PsAbs,
        528 => PsMerge00,
        560 => PsMerge01,
        592 => PsMerge10,
        624 => PsMerge11,
        _ => Unknown,
    }
}

fn decode_table_19(inst: Inst) -> Op {
    use Op::*;
    match inst.subop10() {
        0 => Mcrf,
        16 => Bclr,
        33 => Crnor,
        129 => Crandc,
        150 => Isync,
        193 => Crxor,
        225 => Crnand,
        257 => Crand,
        289 => Creqv,
        417 => Crorc,
        449 => Cror,
        528 => Bcctr,
        _ => Unknown,
    }
}

fn decode_table_31(inst: Inst) -> Op {
    use Op::*;
    // Note: subop10 encodes both XO and X forms. We treat OE as part of the
    // mnemonic suffix (handled at print time), so we mask bit 9 of subop10.
    // To keep the table simple we list canonical (OE=0) subops here.
    let sub = inst.subop10() & 0x1ff; // strip OE bit
    match sub {
        // Compare
        0 => Cmp,
        32 => Cmpl,
        // Arithmetic
        266 => Add, 10 => Addc, 138 => Adde,
        202 => Addze, 234 => Addme,
        40 => Subf, 8 => Subfc, 136 => Subfe,
        200 => Subfze, 232 => Subfme,
        104 => Neg,
        235 => Mullw, 75 => Mulhw, 11 => Mulhwu,
        491 => Divw, 459 => Divwu,
        // Logical / shift
        28 => And, 444 => Or, 316 => Xor,
        476 => Nand, 124 => Nor, 284 => Eqv,
        60 => Andc, 412 => Orc,
        954 => Extsb, 922 => Extsh, 26 => Cntlzw,
        24 => Slw, 536 => Srw, 792 => Sraw, 824 => Srawi,
        // SPR / system
        19 => Mfcr,
        144 => Mtcrf,
        339 => Mfspr,
        467 => Mtspr,
        83 => Mfmsr,
        146 => Mtmsr,
        598 => Sync, 854 => Eieio,
        // Cache
        1014 => Dcbz, 470 => Dcbi, 86 => Dcbf, 54 => Dcbst,
        278 => Dcbt, 246 => Dcbtst, 982 => Icbi,
        // Trap
        4 => Tw,
        // Indexed integer load/store
        23 => Lwzx, 55 => Lwzux,
        87 => Lbzx, 119 => Lbzux,
        279 => Lhzx, 311 => Lhzux,
        343 => Lhax, 375 => Lhaux,
        151 => Stwx, 183 => Stwux,
        215 => Stbx, 247 => Stbux,
        407 => Sthx, 439 => Sthux,
        534 => Lwbrx, 662 => Stwbrx,
        790 => Lhbrx, 918 => Sthbrx,
        // FP indexed load/store
        535 => Lfsx, 567 => Lfsux,
        599 => Lfdx, 631 => Lfdux,
        663 => Stfsx, 695 => Stfsux,
        727 => Stfdx, 759 => Stfdux,
        // Paired-singles indexed
        6 => PsqLx, 38 => PsqLux,
        7 => PsqStx, 39 => PsqStux,
        _ => Unknown,
    }
}

fn decode_table_59(inst: Inst) -> Op {
    use Op::*;
    match inst.subop5() {
        18 => Fdivs,
        20 => Fsubs,
        21 => Fadds,
        24 => Fres,
        25 => Fmuls,
        28 => Fmsubs,
        29 => Fmadds,
        30 => Fnmsubs,
        31 => Fnmadds,
        _ => Unknown,
    }
}

fn decode_table_63(inst: Inst) -> Op {
    use Op::*;
    match inst.subop5() {
        18 => return Fdiv,
        20 => return Fsub,
        21 => return Fadd,
        22 => return Fsqrt,
        23 => return Fsel,
        25 => return Fmul,
        26 => return Frsqrte,
        28 => return Fmsub,
        29 => return Fmadd,
        30 => return Fnmsub,
        31 => return Fnmadd,
        _ => {}
    }
    match inst.subop10() {
        0 => Fcmpu,
        32 => Fcmpo,
        12 => Frsp,
        14 => Fctiw,
        15 => Fctiwz,
        40 => Fneg,
        72 => Fmr,
        136 => Fnabs,
        264 => Fabs,
        583 => Mffs,
        711 => Mtfsf,
        _ => Unknown,
    }
}
