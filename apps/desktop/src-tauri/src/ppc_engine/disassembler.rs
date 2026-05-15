//! Disassembler — converts raw 32-bit words into [`DisasmLine`] entries.
//!
//! Output is intentionally close to GNU `as` mnemonics so users can paste
//! disassembled text back into the assembler.

use serde::{Deserialize, Serialize};

use super::inst::Inst;
use super::tables::{decode, Op};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DisasmLine {
    pub address: u32,
    pub raw: u32,
    pub mnemonic: String,
    pub operands: String,
    /// Optional symbol label that resolves to this address.
    pub label: Option<String>,
}

/// Disassemble a byte slice. `bytes` is interpreted as big-endian u32 words.
pub fn disassemble(bytes: &[u8], base_addr: u32) -> Vec<DisasmLine> {
    let mut out = Vec::with_capacity(bytes.len() / 4);
    let mut addr = base_addr;
    for chunk in bytes.chunks_exact(4) {
        let raw = u32::from_be_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]);
        let inst = Inst(raw);
        let op = decode(inst);
        let mnemonic = format_mnemonic(inst, op);
        let operands = format_operands(inst, op);
        out.push(DisasmLine { address: addr, raw, mnemonic, operands, label: None });
        addr = addr.wrapping_add(4);
    }
    out
}

/// Public so the interpreter can use it to build trace entries.
pub fn format_operands(inst: Inst, op: Op) -> String {
    use Op::*;
    match op {
        // D-form add/sub immediate
        Addi | Addic | AddicDot | Subfic | Mulli => {
            format!("r{}, r{}, {}", inst.rd(), inst.ra(), inst.simm())
        }
        Addis => format!("r{}, r{}, {}", inst.rd(), inst.ra(), inst.simm()),

        // Logical immediate
        Andi | Andis | Ori | Oris | Xori | Xoris => {
            format!("r{}, r{}, 0x{:04X}", inst.ra(), inst.rs(), inst.uimm())
        }

        // 3-reg integer
        Add | Addo | Addc | Adde | Subf | Subfc | Subfe
        | Mullw | Mulhw | Mulhwu | Divw | Divwu => {
            format!("r{}, r{}, r{}", inst.rd(), inst.ra(), inst.rb())
        }
        Addze | Addme | Subfze | Subfme | Neg => {
            format!("r{}, r{}", inst.rd(), inst.ra())
        }

        // Logical / shift register
        And | Or | Xor | Nand | Nor | Eqv | Andc | Orc | Slw | Srw | Sraw => {
            format!("r{}, r{}, r{}", inst.ra(), inst.rs(), inst.rb())
        }
        Srawi => format!("r{}, r{}, {}", inst.ra(), inst.rs(), inst.sh()),
        Extsb | Extsh | Cntlzw => format!("r{}, r{}", inst.ra(), inst.rs()),
        Rlwinm | Rlwnm | Rlwimi => format!(
            "r{}, r{}, {}, {}, {}",
            inst.ra(), inst.rs(), inst.sh(), inst.mb(), inst.me()
        ),

        // Compare
        Cmp | Cmpl => format!("cr{}, r{}, r{}", inst.crfd(), inst.ra(), inst.rb()),
        Cmpi => format!("cr{}, r{}, {}", inst.crfd(), inst.ra(), inst.simm()),
        Cmpli => format!("cr{}, r{}, {}", inst.crfd(), inst.ra(), inst.uimm()),

        // Branches
        B => format!("0x{:X}", inst.li()),
        Bc => format!("{}, {}, 0x{:X}", inst.bo(), inst.bi(), inst.bd()),
        Bclr => format!("{}, {}", inst.bo(), inst.bi()),
        Bcctr => format!("{}, {}", inst.bo(), inst.bi()),

        // CR / SPR
        Mcrf => format!("cr{}, cr{}", inst.crfd(), inst.crfs()),
        Crand | Cror | Crxor | Crnand | Crnor | Creqv | Crandc | Crorc => {
            format!("{}, {}, {}", inst.crbd(), inst.crba(), inst.crbb())
        }
        Mfcr => format!("r{}", inst.rd()),
        Mtcrf => format!("0x{:02X}, r{}", inst.crm(), inst.rs()),
        Mfspr => format!("r{}, {}", inst.rd(), inst.spr()),
        Mtspr => format!("{}, r{}", inst.spr(), inst.rs()),
        Mfmsr => format!("r{}", inst.rd()),
        Mtmsr => format!("r{}", inst.rs()),
        Mfsr => format!("r{}, {}", inst.rd(), inst.ra() as u32 & 0xf),
        Mtsr => format!("{}, r{}", inst.ra() as u32 & 0xf, inst.rs()),
        Mfsrin => format!("r{}, r{}", inst.rd(), inst.rb()),
        Mtsrin => format!("r{}, r{}", inst.rs(), inst.rb()),
        Tlbie => {
            if inst.rs() == 0 {
                format!("r{}", inst.rb())
            } else {
                format!("r{}, r{}", inst.rb(), inst.rs())
            }
        }
        Tlbia | Tlbsync => String::new(),
        Rfi => String::new(),

        // Integer load / store (D-form)
        Lbz | Lbzu | Lhz | Lhzu | Lha | Lhau | Lwz | Lwzu | Lmw
        | Stb | Stbu | Sth | Sthu | Stw | Stwu | Stmw => {
            format!("r{}, {}(r{})", inst.rd(), inst.simm(), inst.ra())
        }
        Lbzx | Lbzux | Lhzx | Lhzux | Lhax | Lhaux | Lwzx | Lwzux
        | Stbx | Stbux | Sthx | Sthux | Stwx | Stwux
        | Lwbrx | Stwbrx | Lhbrx | Sthbrx => {
            format!("r{}, r{}, r{}", inst.rd(), inst.ra(), inst.rb())
        }

        // FP load / store
        Lfs | Lfsu | Lfd | Lfdu | Stfs | Stfsu | Stfd | Stfdu => {
            format!("f{}, {}(r{})", inst.rd(), inst.simm(), inst.ra())
        }
        Lfsx | Lfsux | Lfdx | Lfdux | Stfsx | Stfsux | Stfdx | Stfdux => {
            format!("f{}, r{}, r{}", inst.rd(), inst.ra(), inst.rb())
        }

        // FP arithmetic
        Fadds | Fsubs | Fadd | Fsub | Fdivs | Fdiv => {
            format!("f{}, f{}, f{}", inst.rd(), inst.ra(), inst.rb())
        }
        Fmuls | Fmul => format!("f{}, f{}, f{}", inst.rd(), inst.ra(), inst.rc_reg()),
        Fmadds | Fmsubs | Fnmadds | Fnmsubs | Fmadd | Fmsub | Fnmadd | Fnmsub => {
            format!("f{}, f{}, f{}, f{}", inst.rd(), inst.ra(), inst.rc_reg(), inst.rb())
        }
        Fsqrt | Frsp | Fabs | Fneg | Fnabs | Fmr | Fres | Frsqrte
        | Fctiw | Fctiwz => format!("f{}, f{}", inst.rd(), inst.rb()),
        Fsel => format!("f{}, f{}, f{}, f{}", inst.rd(), inst.ra(), inst.rc_reg(), inst.rb()),
        Fcmpu | Fcmpo => format!("cr{}, f{}, f{}", inst.crfd(), inst.ra(), inst.rb()),
        Mffs => format!("f{}", inst.rd()),
        Mtfsf => format!("0x{:02X}, f{}", inst.fm(), inst.rb()),

        // Paired singles
        PsAdd | PsSub | PsDiv => format!("f{}, f{}, f{}", inst.rd(), inst.ra(), inst.rb()),
        PsMul | PsMuls0 | PsMuls1 => format!("f{}, f{}, f{}", inst.rd(), inst.ra(), inst.rc_reg()),
        PsMadd | PsMsub | PsNmadd | PsNmsub | PsMadds0 | PsMadds1 => format!(
            "f{}, f{}, f{}, f{}",
            inst.rd(), inst.ra(), inst.rc_reg(), inst.rb()
        ),
        PsSum0 | PsSum1 | PsSel => format!(
            "f{}, f{}, f{}, f{}",
            inst.rd(), inst.ra(), inst.rc_reg(), inst.rb()
        ),
        PsMerge00 | PsMerge01 | PsMerge10 | PsMerge11 => {
            format!("f{}, f{}, f{}", inst.rd(), inst.ra(), inst.rb())
        }
        PsAbs | PsNeg | PsNabs | PsMr | PsRes | PsRsqrte => format!("f{}, f{}", inst.rd(), inst.rb()),
        PsCmpu0 | PsCmpu1 | PsCmpo0 | PsCmpo1 => {
            format!("cr{}, f{}, f{}", inst.crfd(), inst.ra(), inst.rb())
        }
        PsqL | PsqLu => format!(
            "f{}, {}(r{}), {}, {}",
            inst.rd(), inst.psq_d(), inst.ra(), inst.w() as u32, inst.i()
        ),
        PsqLx | PsqLux => format!(
            "f{}, r{}, r{}, {}, {}",
            inst.rd(), inst.ra(), inst.rb(), inst.w() as u32, inst.i()
        ),
        PsqSt | PsqStu => format!(
            "f{}, {}(r{}), {}, {}",
            inst.rs(), inst.psq_d(), inst.ra(), inst.w() as u32, inst.i()
        ),
        PsqStx | PsqStux => format!(
            "f{}, r{}, r{}, {}, {}",
            inst.rs(), inst.ra(), inst.rb(), inst.w() as u32, inst.i()
        ),

        // Cache / sync — no operands or one operand
        Dcbz | Dcbi | Dcbf | Dcbst | Dcbt | Dcbtst | Icbi => {
            format!("r{}, r{}", inst.ra(), inst.rb())
        }
        Sync | Isync | Eieio | Sc => String::new(),
        Tw | Twi => format!("{}, r{}, ...", inst.rd(), inst.ra()),

        Unknown => format!("0x{:08X}", inst.raw()),
    }
}

fn format_mnemonic(inst: Inst, op: Op) -> String {
    let mut s = op.mnemonic().to_string();
    // Suffix '.' for Rc on operations that support it.
    if has_rc_suffix(op) && inst.rc() {
        s.push('.');
    }
    s
}

fn has_rc_suffix(op: Op) -> bool {
    use Op::*;
    matches!(
        op,
        Add | Addo | Addc | Adde | Addze | Addme | Subf | Subfc | Subfe | Subfze | Subfme | Neg
        | Mullw | Mulhw | Mulhwu | Divw | Divwu
        | And | Or | Xor | Nand | Nor | Eqv | Andc | Orc
        | Extsb | Extsh | Cntlzw
        | Slw | Srw | Sraw | Srawi | Rlwinm | Rlwnm | Rlwimi
        | Fadd | Fsub | Fmul | Fdiv | Fmadd | Fmsub | Fnmadd | Fnmsub | Fsqrt | Frsp
        | Fabs | Fneg | Fnabs | Fmr | Fres | Frsqrte | Fctiw | Fctiwz
        | Fadds | Fsubs | Fmuls | Fdivs | Fmadds | Fmsubs | Fnmadds | Fnmsubs
    )
}
