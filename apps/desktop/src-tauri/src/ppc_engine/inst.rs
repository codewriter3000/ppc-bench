//! Bit-field accessors for the 32-bit PPC instruction word.
//!
//! Mirrors the `UGeckoInstruction` union in Dolphin's `Gekko.h`. Where Dolphin
//! uses a packed bitfield union, we use explicit accessor methods on a
//! `Copy` wrapper struct. Field semantics and bit positions are identical.
//!
//! PPC fields use IBM big-endian bit numbering (bit 0 is the MSB), so a field
//! "at bits A..B" maps to little-endian shifts as `(31 - B)..=(31 - A)`.

/// A decoded PPC instruction word.
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub struct Inst(pub u32);

impl Inst {
    #[inline] pub fn raw(self) -> u32 { self.0 }

    /// Primary opcode (bits 0..6 in IBM numbering, i.e. top 6 bits).
    #[inline] pub fn opcd(self) -> u32 { (self.0 >> 26) & 0x3f }

    /// Extended opcode for X/XO/XL forms (bits 21..31 IBM, low 10 bits before Rc).
    #[inline] pub fn subop10(self) -> u32 { (self.0 >> 1) & 0x3ff }
    /// Short extended opcode for A-form (bits 26..30 IBM).
    #[inline] pub fn subop5(self) -> u32 { (self.0 >> 1) & 0x1f }

    /// Record bit (bit 31, LSB) — when set, update CR0/CR1 with result.
    #[inline] pub fn rc(self) -> bool { (self.0 & 1) != 0 }
    /// Overflow-enable bit (bit 21 IBM) for OE forms.
    #[inline] pub fn oe(self) -> bool { ((self.0 >> 10) & 1) != 0 }
    /// Link bit on branch instructions (bit 31).
    #[inline] pub fn lk(self) -> bool { (self.0 & 1) != 0 }
    /// Absolute-address bit on branches (bit 30).
    #[inline] pub fn aa(self) -> bool { ((self.0 >> 1) & 1) != 0 }

    /// Destination register (bits 6..11 IBM).
    #[inline] pub fn rd(self) -> usize { ((self.0 >> 21) & 0x1f) as usize }
    /// Source register S — same bits as RD; named differently for store ops.
    #[inline] pub fn rs(self) -> usize { self.rd() }
    /// First source register (bits 11..16 IBM).
    #[inline] pub fn ra(self) -> usize { ((self.0 >> 16) & 0x1f) as usize }
    /// Second source register (bits 16..21 IBM).
    #[inline] pub fn rb(self) -> usize { ((self.0 >> 11) & 0x1f) as usize }
    /// Third source register for fused-multiply forms (bits 21..26 IBM).
    #[inline] pub fn rc_reg(self) -> usize { ((self.0 >> 6) & 0x1f) as usize }

    /// 16-bit sign-extended immediate.
    #[inline] pub fn simm(self) -> i32 { (self.0 as i16) as i32 }
    /// 16-bit zero-extended immediate.
    #[inline] pub fn uimm(self) -> u32 { self.0 & 0xffff }

    /// Branch displacement for B-form (bits 6..30 IBM, sign-extended *4-byte units).
    #[inline]
    pub fn li(self) -> i32 {
        // Bits 6..29 = 24-bit displacement, low 2 bits implicit zero.
        let v = (self.0 & 0x03ff_fffc) as i32;
        // Sign-extend from bit 25.
        (v << 6) >> 6
    }

    /// Conditional branch displacement (bits 16..29 IBM).
    #[inline]
    pub fn bd(self) -> i32 {
        let v = (self.0 & 0x0000_fffc) as i32;
        // Sign-extend from bit 15.
        (v << 16) >> 16
    }

    /// BO field — branch condition options (bits 6..11 IBM).
    #[inline] pub fn bo(self) -> u32 { (self.0 >> 21) & 0x1f }
    /// BI field — CR bit to test (bits 11..16 IBM).
    #[inline] pub fn bi(self) -> u32 { (self.0 >> 16) & 0x1f }

    /// SPR field for mfspr/mtspr (bits 11..21, split halves swapped per PPC ISA).
    #[inline]
    pub fn spr(self) -> u32 {
        let hi = (self.0 >> 16) & 0x1f;
        let lo = (self.0 >> 11) & 0x1f;
        (lo << 5) | hi
    }

    /// CRBD/CRBA/CRBB bit indices for CR ops (bits 6..11, 11..16, 16..21).
    #[inline] pub fn crbd(self) -> u32 { (self.0 >> 21) & 0x1f }
    #[inline] pub fn crba(self) -> u32 { (self.0 >> 16) & 0x1f }
    #[inline] pub fn crbb(self) -> u32 { (self.0 >> 11) & 0x1f }

    /// CRFD/CRFS — CR field destination / source (3-bit, bits 6..9 / 11..14).
    #[inline] pub fn crfd(self) -> u32 { (self.0 >> 23) & 0x7 }
    #[inline] pub fn crfs(self) -> u32 { (self.0 >> 18) & 0x7 }

    /// Mask fields for rlwinm / rlwimi / rlwnm (SH bits 16..21, MB 21..26, ME 26..31).
    #[inline] pub fn sh(self) -> u32 { (self.0 >> 11) & 0x1f }
    #[inline] pub fn mb(self) -> u32 { (self.0 >> 6) & 0x1f }
    #[inline] pub fn me(self) -> u32 { (self.0 >> 1) & 0x1f }

    /// CRM — CR mask field for mtcrf (bits 12..20 IBM).
    #[inline] pub fn crm(self) -> u32 { (self.0 >> 12) & 0xff }

    /// FM — FPSCR mask for mtfsf (bits 7..15 IBM).
    #[inline] pub fn fm(self) -> u32 { (self.0 >> 17) & 0xff }

    /// Paired-singles quantization register index (GQR), bits 12..15.
    #[inline] pub fn i(self) -> u32 { (self.0 >> 12) & 0x7 }
    /// Paired-singles "W" flag — load-as-single-or-paired (bit 16).
    #[inline] pub fn w(self) -> bool { ((self.0 >> 15) & 1) != 0 }
    /// Paired-singles 12-bit signed displacement (bits 20..31).
    #[inline]
    pub fn psq_d(self) -> i32 {
        let v = (self.0 & 0xfff) as i32;
        (v << 20) >> 20
    }
}

/// Helper: sign-extend an `N`-bit value held in a `u32` to `i32`.
#[inline]
pub fn sext(value: u32, bits: u32) -> i32 {
    let shift = 32 - bits;
    ((value << shift) as i32) >> shift
}
