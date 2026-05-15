//! Text assembler (two-pass).
//!
//! Supports the most common Gekko mnemonics in a GNU-as-flavoured syntax.
//! Designed for *learning*, not for assembling real game binaries — corner
//! cases like absolute branches, extended mnemonics (`bne`, `li`, `mr`, …),
//! and `addis r3, r3, ha16(label)` are handled where they're educational.
//!
//! Lines:
//!     [label:] [mnemonic operand1[, operand2[, ...]]] [# comment]
//!
//! Registers:  `rN`, `fN`, `crN` (N = 0..31 / 0..7)
//! Immediates: decimal, `0x...`, `0b...`, or a label name (resolved to address)
//! Memory:     `disp(rA)` or `disp(rA, rB)` for indexed forms (the X-form
//!             alternative `rA, rB` is also accepted)

use serde::{Deserialize, Serialize};

use super::memory::BASE_ADDR;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssembleError {
    pub line: u32,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssembleSourceMapEntry {
    pub line: u32,
    pub start_addr: u32,
    pub byte_len: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssembleResult {
    pub ok: bool,
    pub bytes: Vec<u8>,
    pub base_addr: u32,
    pub symbols: Vec<(String, u32)>,
    pub source_map: Vec<AssembleSourceMapEntry>,
    pub errors: Vec<AssembleError>,
}

/// What a single source line contributes to the output.
enum TokenEntry {
    /// A standard PPC instruction — always encodes to exactly 4 bytes.
    Instruction(String, Vec<String>),
    /// Already-resolved bytes (.string, .float, .double, .zero, etc.).
    Bytes(Vec<u8>),
    /// .long / .word — one 32-bit big-endian word per operand; may reference symbols.
    Longs(Vec<String>),
    /// .short / .hword — one 16-bit big-endian halfword per operand.
    Shorts(Vec<String>),
    /// .byte — one byte per operand.
    ByteOps(Vec<String>),
}

/// Assemble `source` into a big-endian byte stream starting at [`BASE_ADDR`].
pub fn assemble(source: &str) -> AssembleResult {
    let mut errors: Vec<AssembleError> = Vec::new();
    let mut symbols: Vec<(String, u32)> = Vec::new();
    // (source_line_no, addr, entry)
    let mut tokens: Vec<(u32, u32, TokenEntry)> = Vec::new();

    // ── Pass 1: tokenize, gather labels, compute sizes ────────────────
    let mut pc = BASE_ADDR;
    for (line_no, raw_line) in source.lines().enumerate() {
        let line_no = line_no as u32 + 1;
        let mut line = strip_comment(raw_line).trim().to_string();
        if line.is_empty() {
            continue;
        }
        // Label detection: ':' only introduces a label when it appears before
        // any whitespace (so we don't confuse memory operands or string contents).
        let colon_pos = line.find(':');
        let first_ws  = line.find(char::is_whitespace);
        if let Some(idx) = colon_pos {
            if first_ws.map_or(true, |sp| idx < sp) {
                let lbl = line[..idx].trim();
                if !lbl.is_empty() {
                    if !is_ident(lbl) {
                        errors.push(AssembleError {
                            line: line_no,
                            message: format!("invalid label '{}'", lbl),
                        });
                        continue;
                    }
                    symbols.push((lbl.to_string(), pc));
                    line = line[idx + 1..].trim().to_string();
                    if line.is_empty() {
                        continue;
                    }
                }
            }
        }

        // Split off mnemonic, keep rest raw for data directives.
        let first_ws2 = line.find(char::is_whitespace);
        let mnemonic  = match first_ws2 { Some(i) => line[..i].to_lowercase(), None => line.to_lowercase() };
        let rest      = match first_ws2 { Some(i) => line[i..].trim(), None => "" };

        // Section directives — no bytes emitted.
        match mnemonic.as_str() {
            ".data" | ".text" | ".section" | ".rodata" | ".bss" => continue,
            _ => {}
        }

        // Data directives.
        match mnemonic.as_str() {
            ".long" | ".word" | ".int" | ".4byte" => {
                let ops: Vec<String> = rest.split(',')
                    .map(|s| s.trim().to_string())
                    .filter(|s| !s.is_empty())
                    .collect();
                let size = (ops.len() as u32) * 4;
                tokens.push((line_no, pc, TokenEntry::Longs(ops)));
                pc = pc.wrapping_add(size);
                continue;
            }
            ".short" | ".hword" | ".2byte" => {
                let ops: Vec<String> = rest.split(',')
                    .map(|s| s.trim().to_string())
                    .filter(|s| !s.is_empty())
                    .collect();
                let size = (ops.len() as u32) * 2;
                tokens.push((line_no, pc, TokenEntry::Shorts(ops)));
                pc = pc.wrapping_add(size);
                continue;
            }
            ".byte" => {
                let ops: Vec<String> = rest.split(',')
                    .map(|s| s.trim().to_string())
                    .filter(|s| !s.is_empty())
                    .collect();
                let size = ops.len() as u32;
                tokens.push((line_no, pc, TokenEntry::ByteOps(ops)));
                pc = pc.wrapping_add(size);
                continue;
            }
            ".float" | ".single" => {
                let mut data_bytes: Vec<u8> = Vec::new();
                let mut ok = true;
                for op in rest.split(',') {
                    let op = op.trim();
                    if op.is_empty() { continue; }
                    match op.parse::<f32>() {
                        Ok(v)  => data_bytes.extend_from_slice(&v.to_bits().to_be_bytes()),
                        Err(_) => {
                            errors.push(AssembleError { line: line_no, message: format!("invalid float '{}'", op) });
                            ok = false;
                        }
                    }
                }
                if ok {
                    let size = data_bytes.len() as u32;
                    tokens.push((line_no, pc, TokenEntry::Bytes(data_bytes)));
                    pc = pc.wrapping_add(size);
                }
                continue;
            }
            ".double" => {
                let mut data_bytes: Vec<u8> = Vec::new();
                let mut ok = true;
                for op in rest.split(',') {
                    let op = op.trim();
                    if op.is_empty() { continue; }
                    match op.parse::<f64>() {
                        Ok(v)  => data_bytes.extend_from_slice(&v.to_bits().to_be_bytes()),
                        Err(_) => {
                            errors.push(AssembleError { line: line_no, message: format!("invalid double '{}'", op) });
                            ok = false;
                        }
                    }
                }
                if ok {
                    let size = data_bytes.len() as u32;
                    tokens.push((line_no, pc, TokenEntry::Bytes(data_bytes)));
                    pc = pc.wrapping_add(size);
                }
                continue;
            }
            ".string" | ".asciz" => {
                match parse_string_literal(rest) {
                    Ok(s) => {
                        let mut data_bytes: Vec<u8> = s.into_bytes();
                        data_bytes.push(0); // null terminator
                        let size = data_bytes.len() as u32;
                        tokens.push((line_no, pc, TokenEntry::Bytes(data_bytes)));
                        pc = pc.wrapping_add(size);
                    }
                    Err(e) => errors.push(AssembleError { line: line_no, message: e }),
                }
                continue;
            }
            ".ascii" => {
                match parse_string_literal(rest) {
                    Ok(s) => {
                        let data_bytes: Vec<u8> = s.into_bytes();
                        let size = data_bytes.len() as u32;
                        tokens.push((line_no, pc, TokenEntry::Bytes(data_bytes)));
                        pc = pc.wrapping_add(size);
                    }
                    Err(e) => errors.push(AssembleError { line: line_no, message: e }),
                }
                continue;
            }
            ".zero" | ".space" => {
                match rest.trim().parse::<u32>() {
                    Ok(n) => {
                        tokens.push((line_no, pc, TokenEntry::Bytes(vec![0u8; n as usize])));
                        pc = pc.wrapping_add(n);
                    }
                    Err(_) => errors.push(AssembleError { line: line_no, message: format!("invalid count '{}'", rest) }),
                }
                continue;
            }
            ".align" | ".balign" => {
                if let Ok(n) = rest.trim().parse::<u32>() {
                    let align = if mnemonic == ".align" { 1u32 << n.min(16) } else { n };
                    if align > 1 {
                        let rem = pc % align;
                        if rem != 0 {
                            let pad = align - rem;
                            tokens.push((line_no, pc, TokenEntry::Bytes(vec![0u8; pad as usize])));
                            pc = pc.wrapping_add(pad);
                        }
                    }
                } else {
                    errors.push(AssembleError { line: line_no, message: format!("invalid alignment '{}'", rest) });
                }
                continue;
            }
            _ => {}
        }

        // Regular PPC instruction.
        let ops: Vec<String> = if rest.is_empty() {
            Vec::new()
        } else {
            rest.split(',').map(|s| s.trim().to_string()).collect()
        };
        tokens.push((line_no, pc, TokenEntry::Instruction(mnemonic, ops)));
        pc = pc.wrapping_add(4);
    }

    // ── Pass 2: encode ────────────────────────────────────────────────
    let mut bytes: Vec<u8> = Vec::new();
    for (line_no, addr, entry) in &tokens {
        match entry {
            TokenEntry::Instruction(mnemonic, operands) => {
                match encode(*addr, mnemonic, operands, &symbols) {
                    Ok(word) => bytes.extend_from_slice(&word.to_be_bytes()),
                    Err(msg) => {
                        errors.push(AssembleError { line: *line_no, message: msg });
                        bytes.extend_from_slice(&0u32.to_be_bytes());
                    }
                }
            }
            TokenEntry::Bytes(data) => bytes.extend_from_slice(data),
            TokenEntry::Longs(ops) => {
                for op in ops {
                    match parse_imm(op, &symbols) {
                        Ok(v)  => bytes.extend_from_slice(&(v as u32).to_be_bytes()),
                        Err(e) => {
                            errors.push(AssembleError { line: *line_no, message: e });
                            bytes.extend_from_slice(&0u32.to_be_bytes());
                        }
                    }
                }
            }
            TokenEntry::Shorts(ops) => {
                for op in ops {
                    match parse_imm(op, &symbols) {
                        Ok(v)  => bytes.extend_from_slice(&(v as u16).to_be_bytes()),
                        Err(e) => {
                            errors.push(AssembleError { line: *line_no, message: e });
                            bytes.extend_from_slice(&0u16.to_be_bytes());
                        }
                    }
                }
            }
            TokenEntry::ByteOps(ops) => {
                for op in ops {
                    match parse_imm(op, &symbols) {
                        Ok(v)  => bytes.push(v as u8),
                        Err(e) => {
                            errors.push(AssembleError { line: *line_no, message: e });
                            bytes.push(0);
                        }
                    }
                }
            }
        }
    }

    let source_map = tokens.iter()
        .filter_map(|(line_no, addr, entry)| {
            let byte_len = token_byte_len(entry);
            if byte_len == 0 {
                None
            } else {
                Some(AssembleSourceMapEntry {
                    line: *line_no,
                    start_addr: *addr,
                    byte_len,
                })
            }
        })
        .collect();

    AssembleResult {
        ok: errors.is_empty(),
        bytes,
        base_addr: BASE_ADDR,
        symbols,
        source_map,
        errors,
    }
}

// ── Helpers ───────────────────────────────────────────────────────────

fn token_byte_len(entry: &TokenEntry) -> u32 {
    match entry {
        TokenEntry::Instruction(_, _) => 4,
        TokenEntry::Bytes(data) => data.len() as u32,
        TokenEntry::Longs(ops) => (ops.len() as u32) * 4,
        TokenEntry::Shorts(ops) => (ops.len() as u32) * 2,
        TokenEntry::ByteOps(ops) => ops.len() as u32,
    }
}

fn strip_comment(line: &str) -> &str {
    let mut in_str = false;
    let mut escape = false;
    for (i, c) in line.char_indices() {
        if escape { escape = false; continue; }
        if in_str {
            if c == '\\' { escape = true; }
            else if c == '"' { in_str = false; }
        } else {
            match c {
                '"'      => in_str = true,
                '#' | ';' => return &line[..i],
                _        => {}
            }
        }
    }
    line
}

/// Parse a double-quoted string literal, processing standard escape sequences.
fn parse_string_literal(s: &str) -> Result<String, String> {
    let s = s.trim();
    if !s.starts_with('"') {
        return Err(format!("expected string literal, got '{}'", s));
    }
    let mut result = String::new();
    let mut chars  = s[1..].chars();
    let mut closed = false;
    while let Some(c) = chars.next() {
        if c == '"' { closed = true; break; }
        if c == '\\' {
            match chars.next() {
                Some('n')  => result.push('\n'),
                Some('t')  => result.push('\t'),
                Some('r')  => result.push('\r'),
                Some('0')  => result.push('\0'),
                Some('\\') => result.push('\\'),
                Some('"')  => result.push('"'),
                Some(c)    => { result.push('\\'); result.push(c); }
                None       => return Err("unterminated string escape".to_string()),
            }
        } else {
            result.push(c);
        }
    }
    if !closed {
        return Err("unterminated string literal".to_string());
    }
    Ok(result)
}

fn is_ident(s: &str) -> bool {
    let mut chars = s.chars();
    match chars.next() {
        Some(c) if c.is_ascii_alphabetic() || c == '_' || c == '.' => {}
        _ => return false,
    }
    chars.all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '.')
}

fn split_mnemonic(line: &str) -> (String, Vec<String>) {
    let mut it = line.splitn(2, char::is_whitespace);
    let mnemonic = it.next().unwrap_or("").to_lowercase();
    let rest = it.next().unwrap_or("").trim();
    let ops = if rest.is_empty() {
        Vec::new()
    } else {
        rest.split(',').map(|s| s.trim().to_string()).collect()
    };
    (mnemonic, ops)
}

fn parse_reg(s: &str, prefix: char) -> Result<u32, String> {
    let s = s.trim();
    if !s.starts_with(prefix) {
        return Err(format!("expected register starting with '{}', got '{}'", prefix, s));
    }
    let n: u32 = s[1..].parse().map_err(|_| format!("bad register '{}'", s))?;
    if n >= 32 {
        return Err(format!("register out of range: {}", s));
    }
    Ok(n)
}

fn parse_cr(s: &str) -> Result<u32, String> {
    let s = s.trim();
    if let Some(rest) = s.strip_prefix("cr") {
        let n: u32 = rest.parse().map_err(|_| format!("bad CR field '{}'", s))?;
        if n >= 8 { return Err(format!("CR field out of range: {}", s)); }
        return Ok(n);
    }
    Err(format!("expected crN, got '{}'", s))
}

fn parse_imm(s: &str, symbols: &[(String, u32)]) -> Result<i64, String> {
    let s = s.trim();
    if let Some(hex) = s.strip_prefix("0x").or_else(|| s.strip_prefix("0X")) {
        return i64::from_str_radix(hex, 16).map_err(|e| e.to_string());
    }
    if let Some(neg_hex) = s.strip_prefix("-0x").or_else(|| s.strip_prefix("-0X")) {
        let v = i64::from_str_radix(neg_hex, 16).map_err(|e| e.to_string())?;
        return Ok(-v);
    }
    if let Some(bin) = s.strip_prefix("0b").or_else(|| s.strip_prefix("0B")) {
        return i64::from_str_radix(bin, 2).map_err(|e| e.to_string());
    }
    if let Ok(v) = s.parse::<i64>() {
        return Ok(v);
    }
    if let Some((_, addr)) = symbols.iter().find(|(n, _)| n == s) {
        return Ok(*addr as i64);
    }
    Err(format!("can't parse immediate '{}'", s))
}

/// Parse `disp(rA)` or `disp(rA, rB)`.
fn parse_mem(s: &str, symbols: &[(String, u32)]) -> Result<(i64, u32, Option<u32>), String> {
    let s = s.trim();
    let lp = s.find('(').ok_or_else(|| format!("expected '(' in '{}'", s))?;
    let rp = s.rfind(')').ok_or_else(|| format!("expected ')' in '{}'", s))?;
    let disp = parse_imm(&s[..lp], symbols)?;
    let inside = &s[lp + 1..rp];
    let parts: Vec<&str> = inside.split(',').map(|p| p.trim()).collect();
    let ra = parse_reg(parts[0], 'r')?;
    let rb = if parts.len() > 1 { Some(parse_reg(parts[1], 'r')?) } else { None };
    Ok((disp, ra, rb))
}

fn parse_u4_imm(s: &str, symbols: &[(String, u32)], what: &str) -> Result<u32, String> {
    let value = parse_imm(s, symbols)?;
    if !(0..=15).contains(&value) {
        return Err(format!("{} out of range: {}", what, value));
    }
    Ok(value as u32)
}

// ── Encoders ──────────────────────────────────────────────────────────

#[inline] fn d_form(opcd: u32, rd: u32, ra: u32, simm: i32) -> u32 {
    (opcd << 26) | (rd << 21) | (ra << 16) | ((simm as u32) & 0xffff)
}
#[inline] fn x_form(opcd: u32, rd: u32, ra: u32, rb: u32, subop: u32, rc: u32) -> u32 {
    (opcd << 26) | (rd << 21) | (ra << 16) | (rb << 11) | (subop << 1) | (rc & 1)
}
#[inline] fn xfx_mfspr(rd: u32, spr: u32) -> u32 {
    let lo = spr & 0x1f;
    let hi = (spr >> 5) & 0x1f;
    x_form(31, rd, lo, hi, 339, 0)
}
#[inline] fn xfx_mtspr(rs: u32, spr: u32) -> u32 {
    let lo = spr & 0x1f;
    let hi = (spr >> 5) & 0x1f;
    x_form(31, rs, lo, hi, 467, 0)
}
#[inline] fn b_form(li: i32, aa: bool, lk: bool) -> u32 {
    (18 << 26) | ((li as u32) & 0x03ff_fffc) | ((aa as u32) << 1) | (lk as u32)
}
#[inline] fn bc_form(bo: u32, bi: u32, bd: i32, aa: bool, lk: bool) -> u32 {
    (16 << 26) | (bo << 21) | (bi << 16) | ((bd as u32) & 0xfffc) | ((aa as u32) << 1) | (lk as u32)
}

fn rc_from(mnemonic: &str) -> (String, bool) {
    if let Some(stripped) = mnemonic.strip_suffix('.') {
        (stripped.to_string(), true)
    } else {
        (mnemonic.to_string(), false)
    }
}

fn encode(addr: u32, mnemonic: &str, ops: &[String], symbols: &[(String, u32)]) -> Result<u32, String> {
    let (base, rc_bit) = rc_from(mnemonic);
    let rc_u = rc_bit as u32;

    // Extended/aliased mnemonics handled first.
    match base.as_str() {
        "nop" => return Ok(0x6000_0000), // ori r0, r0, 0
        "li" => {
            need(ops, 2, "li")?;
            let rd = parse_reg(&ops[0], 'r')?;
            let imm = parse_imm(&ops[1], symbols)? as i32;
            return Ok(d_form(14, rd, 0, imm));
        }
        "lis" => {
            need(ops, 2, "lis")?;
            let rd = parse_reg(&ops[0], 'r')?;
            let imm = parse_imm(&ops[1], symbols)? as i32;
            return Ok(d_form(15, rd, 0, imm));
        }
        "mr" => {
            need(ops, 2, "mr")?;
            let ra = parse_reg(&ops[0], 'r')?;
            let rs = parse_reg(&ops[1], 'r')?;
            return Ok(x_form(31, rs, ra, rs, 444, rc_u)); // or rA, rS, rS
        }
        "blr" => return Ok(0x4E80_0020),
        "bctr" => return Ok(0x4E80_0420),
        "blrl" => return Ok(0x4E80_0021),
        "trap" => return Ok(0x7FE0_0008),
        "b" | "bl" | "ba" | "bla" => {
            need(ops, 1, &base)?;
            let target = parse_imm(&ops[0], symbols)? as i64;
            let aa = base.ends_with('a');
            let lk = base.starts_with("bl");
            let disp = if aa { target as i32 } else { (target - addr as i64) as i32 };
            return Ok(b_form(disp, aa, lk));
        }
        "beq" | "bne" | "blt" | "bgt" | "ble" | "bge" | "bso" | "bns" => {
            need_at_least(ops, 1, &base)?;
            // Optional leading "crN," prefix; default cr0.
            let (crf, target_str) = if ops.len() == 2 {
                (parse_cr(&ops[0])?, &ops[1])
            } else {
                (0u32, &ops[0])
            };
            let (bo, bi_off) = match base.as_str() {
                "beq" => (12, 2),
                "bne" => (4, 2),
                "blt" => (12, 0),
                "bge" => (4, 0),
                "bgt" => (12, 1),
                "ble" => (4, 1),
                "bso" => (12, 3),
                "bns" => (4, 3),
                _ => unreachable!(),
            };
            let bi = crf * 4 + bi_off;
            let target = parse_imm(target_str, symbols)? as i64;
            let disp = (target - addr as i64) as i32;
            return Ok(bc_form(bo, bi, disp, false, false));
        }
        _ => {}
    }

    // Canonical mnemonics.
    match base.as_str() {
        // D-form arithmetic
        "addi"  => d3i(ops, symbols, 14),
        "addis" => d3i(ops, symbols, 15),
        "addic" => d3i(ops, symbols, 12),
        "addic." => d3i(ops, symbols, 13),
        "subfic" => d3i(ops, symbols, 8),
        "mulli"  => d3i(ops, symbols, 7),
        "ori"  => di_log(ops, symbols, 24),
        "oris" => di_log(ops, symbols, 25),
        "xori" => di_log(ops, symbols, 26),
        "xoris" => di_log(ops, symbols, 27),
        "andi." => di_log(ops, symbols, 28),
        "andis." => di_log(ops, symbols, 29),
        "cmpi" => cmp_i(ops, symbols, false),
        "cmpli" => cmp_i(ops, symbols, true),

        // X-form arithmetic (opcd=31)
        "add"  => xo_rrr(ops, 266, rc_u),
        "addc" => xo_rrr(ops, 10, rc_u),
        "adde" => xo_rrr(ops, 138, rc_u),
        "subf" => xo_rrr(ops, 40, rc_u),
        "subfc" => xo_rrr(ops, 8, rc_u),
        "mullw" => xo_rrr(ops, 235, rc_u),
        "mulhw" => xo_rrr(ops, 75, rc_u),
        "mulhwu" => xo_rrr(ops, 11, rc_u),
        "divw" => xo_rrr(ops, 491, rc_u),
        "divwu" => xo_rrr(ops, 459, rc_u),
        "neg" => xo_rr(ops, 104, rc_u),
        "addze" => xo_rr(ops, 202, rc_u),
        "addme" => xo_rr(ops, 234, rc_u),
        "subfze" => xo_rr(ops, 200, rc_u),
        "subfme" => xo_rr(ops, 232, rc_u),

        // Logical / shift (X-form, RA,RS,RB layout)
        "and" => x_log(ops, 28, rc_u),
        "or"  => x_log(ops, 444, rc_u),
        "xor" => x_log(ops, 316, rc_u),
        "nand" => x_log(ops, 476, rc_u),
        "nor"  => x_log(ops, 124, rc_u),
        "eqv"  => x_log(ops, 284, rc_u),
        "andc" => x_log(ops, 60, rc_u),
        "orc"  => x_log(ops, 412, rc_u),
        "slw"  => x_log(ops, 24, rc_u),
        "srw"  => x_log(ops, 536, rc_u),
        "sraw" => x_log(ops, 792, rc_u),
        "extsb" => x_log_unary(ops, 954, rc_u),
        "extsh" => x_log_unary(ops, 922, rc_u),
        "cntlzw" => x_log_unary(ops, 26, rc_u),
        "srawi" => {
            need(ops, 3, "srawi")?;
            let ra = parse_reg(&ops[0], 'r')?;
            let rs = parse_reg(&ops[1], 'r')?;
            let sh = parse_imm(&ops[2], symbols)? as u32 & 0x1f;
            Ok(x_form(31, rs, ra, sh, 824, rc_u))
        }
        "rlwinm" | "rlwnm" | "rlwimi" => {
            need(ops, 5, &base)?;
            let ra = parse_reg(&ops[0], 'r')?;
            let rs = parse_reg(&ops[1], 'r')?;
            let sh = parse_imm(&ops[2], symbols)? as u32 & 0x1f;
            let mb = parse_imm(&ops[3], symbols)? as u32 & 0x1f;
            let me = parse_imm(&ops[4], symbols)? as u32 & 0x1f;
            let opcd = match base.as_str() {
                "rlwimi" => 20,
                "rlwinm" => 21,
                "rlwnm"  => 23,
                _ => unreachable!(),
            };
            Ok((opcd << 26) | (rs << 21) | (ra << 16) | (sh << 11) | (mb << 6) | (me << 1) | rc_u)
        }

        // Compare X-form
        "cmp" | "cmpl" => {
            need_at_least(ops, 3, &base)?;
            let crf = parse_cr(&ops[0])?;
            let ra = parse_reg(&ops[1], 'r')?;
            let rb = parse_reg(&ops[2], 'r')?;
            let subop = if base == "cmp" { 0 } else { 32 };
            Ok(x_form(31, crf << 2, ra, rb, subop, 0))
        }

        // Integer load/store D-form
        "lbz" => mem_d(ops, symbols, 34),
        "lbzu" => mem_d(ops, symbols, 35),
        "lhz" => mem_d(ops, symbols, 40),
        "lhzu" => mem_d(ops, symbols, 41),
        "lha" => mem_d(ops, symbols, 42),
        "lhau" => mem_d(ops, symbols, 43),
        "lwz" => mem_d(ops, symbols, 32),
        "lwzu" => mem_d(ops, symbols, 33),
        "stb" => mem_d(ops, symbols, 38),
        "stbu" => mem_d(ops, symbols, 39),
        "sth" => mem_d(ops, symbols, 44),
        "sthu" => mem_d(ops, symbols, 45),
        "stw" => mem_d(ops, symbols, 36),
        "stwu" => mem_d(ops, symbols, 37),
        "lmw" => mem_d(ops, symbols, 46),
        "stmw" => mem_d(ops, symbols, 47),

        // FP load/store D-form
        "lfs" => fmem_d(ops, symbols, 48),
        "lfsu" => fmem_d(ops, symbols, 49),
        "lfd" => fmem_d(ops, symbols, 50),
        "lfdu" => fmem_d(ops, symbols, 51),
        "stfs" => fmem_d(ops, symbols, 52),
        "stfsu" => fmem_d(ops, symbols, 53),
        "stfd" => fmem_d(ops, symbols, 54),
        "stfdu" => fmem_d(ops, symbols, 55),

        // FP arithmetic — single (opcd=59)
        "fadds"   => fa3(ops, 59, 21, rc_u),
        "fsubs"   => fa3(ops, 59, 20, rc_u),
        "fmuls"   => fa_mul(ops, 59, 25, rc_u),
        "fdivs"   => fa3(ops, 59, 18, rc_u),
        "fmadds"  => fa4(ops, 59, 29, rc_u),
        "fmsubs"  => fa4(ops, 59, 28, rc_u),
        "fnmadds" => fa4(ops, 59, 31, rc_u),
        "fnmsubs" => fa4(ops, 59, 30, rc_u),

        // FP arithmetic — double (opcd=63)
        "fadd"  => fa3(ops, 63, 21, rc_u),
        "fsub"  => fa3(ops, 63, 20, rc_u),
        "fmul"  => fa_mul(ops, 63, 25, rc_u),
        "fdiv"  => fa3(ops, 63, 18, rc_u),
        "fmadd" => fa4(ops, 63, 29, rc_u),
        "fmsub" => fa4(ops, 63, 28, rc_u),
        "fnmadd" => fa4(ops, 63, 31, rc_u),
        "fnmsub" => fa4(ops, 63, 30, rc_u),
        "fsqrt" => fa_unary(ops, 63, 22, rc_u),
        "frsp"  => fx_unary(ops, 63, 12, rc_u),
        "fabs"  => fx_unary(ops, 63, 264, rc_u),
        "fneg"  => fx_unary(ops, 63, 40, rc_u),
        "fmr"   => fx_unary(ops, 63, 72, rc_u),

        // SPR / system
        "mfspr" => {
            need(ops, 2, "mfspr")?;
            let rd = parse_reg(&ops[0], 'r')?;
            let spr = parse_imm(&ops[1], symbols)? as u32;
            Ok(xfx_mfspr(rd, spr))
        }
        "mtspr" => {
            need(ops, 2, "mtspr")?;
            let spr = parse_imm(&ops[0], symbols)? as u32;
            let rs = parse_reg(&ops[1], 'r')?;
            Ok(xfx_mtspr(rs, spr))
        }
        "mflr" => {
            need(ops, 1, "mflr")?;
            Ok(xfx_mfspr(parse_reg(&ops[0], 'r')?, 8))
        }
        "mtlr" => {
            need(ops, 1, "mtlr")?;
            Ok(xfx_mtspr(parse_reg(&ops[0], 'r')?, 8))
        }
        "mfctr" => {
            need(ops, 1, "mfctr")?;
            Ok(xfx_mfspr(parse_reg(&ops[0], 'r')?, 9))
        }
        "mtctr" => {
            need(ops, 1, "mtctr")?;
            Ok(xfx_mtspr(parse_reg(&ops[0], 'r')?, 9))
        }
        "mfcr" => {
            need(ops, 1, "mfcr")?;
            Ok(x_form(31, parse_reg(&ops[0], 'r')?, 0, 0, 19, 0))
        }
        "mfsr" => {
            need(ops, 2, "mfsr")?;
            let rd = parse_reg(&ops[0], 'r')?;
            let sr = parse_u4_imm(&ops[1], symbols, "segment register")?;
            Ok(x_form(31, rd, sr, 0, 595, 0))
        }
        "mtsr" => {
            need(ops, 2, "mtsr")?;
            let sr = parse_u4_imm(&ops[0], symbols, "segment register")?;
            let rs = parse_reg(&ops[1], 'r')?;
            Ok(x_form(31, rs, sr, 0, 210, 0))
        }
        "mfsrin" => {
            need(ops, 2, "mfsrin")?;
            let rd = parse_reg(&ops[0], 'r')?;
            let rb = parse_reg(&ops[1], 'r')?;
            Ok(x_form(31, rd, 0, rb, 659, 0))
        }
        "mtsrin" => {
            need(ops, 2, "mtsrin")?;
            let rs = parse_reg(&ops[0], 'r')?;
            let rb = parse_reg(&ops[1], 'r')?;
            Ok(x_form(31, rs, 0, rb, 242, 0))
        }
        "sync" => Ok(x_form(31, 0, 0, 0, 598, 0)),
        "isync" => Ok((19 << 26) | (150 << 1)),
        "rfi" => Ok((19 << 26) | (50 << 1)),
        "sc" => Ok(0x4400_0002),
        "tlbie" => {
            if ops.len() == 1 {
                let rb = parse_reg(&ops[0], 'r')?;
                Ok(x_form(31, 0, 0, rb, 306, 0))
            } else if ops.len() == 2 {
                let rb = parse_reg(&ops[0], 'r')?;
                let rs = parse_reg(&ops[1], 'r')?;
                Ok(x_form(31, rs, 0, rb, 306, 0))
            } else {
                Err(format!("tlbie expects 1 or 2 operands, got {}", ops.len()))
            }
        }
        "tlbia" => Ok(x_form(31, 0, 0, 0, 370, 0)),
        "tlbsync" => Ok(x_form(31, 0, 0, 0, 566, 0)),

        // Paired-singles (subset)
        "ps_add" => fa3(ops, 4, 21, rc_u),
        "ps_sub" => fa3(ops, 4, 20, rc_u),
        "ps_mul" => fa_mul(ops, 4, 25, rc_u),
        "ps_div" => fa3(ops, 4, 18, rc_u),
        "ps_madd" => fa4(ops, 4, 29, rc_u),
        "ps_msub" => fa4(ops, 4, 28, rc_u),
        "ps_mr" => fx_unary(ops, 4, 72, rc_u),
        "ps_neg" => fx_unary(ops, 4, 40, rc_u),
        "ps_abs" => fx_unary(ops, 4, 264, rc_u),
        "ps_merge00" => ps_merge(ops, 528, rc_u),
        "ps_merge01" => ps_merge(ops, 560, rc_u),
        "ps_merge10" => ps_merge(ops, 592, rc_u),
        "ps_merge11" => ps_merge(ops, 624, rc_u),

        _ => Err(format!("unknown mnemonic '{}'", mnemonic)),
    }
}

#[cfg(test)]
mod tests {
    use super::assemble;
    use crate::ppc_engine::disassembler::disassemble;

    #[test]
    fn assembles_and_disassembles_segment_register_family() {
        let result = assemble(
            "mtsr 0, r3\n\
             mfsr r4, 0\n\
             mtsrin r5, r6\n\
             mfsrin r7, r8\n\
             tlbie r9\n\
             tlbie r10, r11\n\
             tlbsync\n\
             tlbia\n",
        );

        assert!(result.ok, "assembly failed: {:?}", result.errors);

        let words: Vec<u32> = result
            .bytes
            .chunks_exact(4)
            .map(|chunk| u32::from_be_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]))
            .collect();
        assert_eq!(
            words,
            vec![
                0x7C60_01A4,
                0x7C80_04A6,
                0x7CA0_31E4,
                0x7CE0_4526,
                0x7C00_4A64,
                0x7D60_5264,
                0x7C00_046C,
                0x7C00_02E4,
            ]
        );

        let disasm = disassemble(&result.bytes, result.base_addr);
        let rendered: Vec<(String, String)> = disasm
            .into_iter()
            .map(|line| (line.mnemonic, line.operands))
            .collect();
        assert_eq!(
            rendered,
            vec![
                ("mtsr".to_string(), "0, r3".to_string()),
                ("mfsr".to_string(), "r4, 0".to_string()),
                ("mtsrin".to_string(), "r5, r6".to_string()),
                ("mfsrin".to_string(), "r7, r8".to_string()),
                ("tlbie".to_string(), "r9".to_string()),
                ("tlbie".to_string(), "r10, r11".to_string()),
                ("tlbsync".to_string(), String::new()),
                ("tlbia".to_string(), String::new()),
            ]
        );
    }
}

fn need(ops: &[String], n: usize, m: &str) -> Result<(), String> {
    if ops.len() != n { Err(format!("{} expects {} operands, got {}", m, n, ops.len())) } else { Ok(()) }
}
fn need_at_least(ops: &[String], n: usize, m: &str) -> Result<(), String> {
    if ops.len() < n { Err(format!("{} expects at least {} operands", m, n)) } else { Ok(()) }
}

fn d3i(ops: &[String], symbols: &[(String, u32)], opcd: u32) -> Result<u32, String> {
    need(ops, 3, "D-form")?;
    let rd = parse_reg(&ops[0], 'r')?;
    let ra = parse_reg(&ops[1], 'r')?;
    let imm = parse_imm(&ops[2], symbols)? as i32;
    Ok(d_form(opcd, rd, ra, imm))
}
fn di_log(ops: &[String], symbols: &[(String, u32)], opcd: u32) -> Result<u32, String> {
    need(ops, 3, "D-form logical")?;
    let ra = parse_reg(&ops[0], 'r')?;
    let rs = parse_reg(&ops[1], 'r')?;
    let imm = parse_imm(&ops[2], symbols)? as i32;
    Ok(d_form(opcd, rs, ra, imm)) // note: RA in the "rd" slot per ISA
}
fn cmp_i(ops: &[String], symbols: &[(String, u32)], unsigned: bool) -> Result<u32, String> {
    need_at_least(ops, 3, "cmp[l]i")?;
    let crf = parse_cr(&ops[0])?;
    let ra = parse_reg(&ops[1], 'r')?;
    let imm = parse_imm(&ops[2], symbols)? as i32;
    let opcd = if unsigned { 10 } else { 11 };
    Ok(d_form(opcd, crf << 2, ra, imm))
}
fn xo_rrr(ops: &[String], subop: u32, rc: u32) -> Result<u32, String> {
    need(ops, 3, "X-form 3-reg")?;
    let rd = parse_reg(&ops[0], 'r')?;
    let ra = parse_reg(&ops[1], 'r')?;
    let rb = parse_reg(&ops[2], 'r')?;
    Ok(x_form(31, rd, ra, rb, subop, rc))
}
fn xo_rr(ops: &[String], subop: u32, rc: u32) -> Result<u32, String> {
    need(ops, 2, "X-form 2-reg")?;
    let rd = parse_reg(&ops[0], 'r')?;
    let ra = parse_reg(&ops[1], 'r')?;
    Ok(x_form(31, rd, ra, 0, subop, rc))
}
fn x_log(ops: &[String], subop: u32, rc: u32) -> Result<u32, String> {
    need(ops, 3, "X-form logical")?;
    let ra = parse_reg(&ops[0], 'r')?;
    let rs = parse_reg(&ops[1], 'r')?;
    let rb = parse_reg(&ops[2], 'r')?;
    Ok(x_form(31, rs, ra, rb, subop, rc))
}
fn x_log_unary(ops: &[String], subop: u32, rc: u32) -> Result<u32, String> {
    need(ops, 2, "X-form unary")?;
    let ra = parse_reg(&ops[0], 'r')?;
    let rs = parse_reg(&ops[1], 'r')?;
    Ok(x_form(31, rs, ra, 0, subop, rc))
}
fn mem_d(ops: &[String], symbols: &[(String, u32)], opcd: u32) -> Result<u32, String> {
    need(ops, 2, "mem D-form")?;
    let rd = parse_reg(&ops[0], 'r')?;
    let (disp, ra, _) = parse_mem(&ops[1], symbols)?;
    Ok(d_form(opcd, rd, ra, disp as i32))
}
fn fmem_d(ops: &[String], symbols: &[(String, u32)], opcd: u32) -> Result<u32, String> {
    need(ops, 2, "fp mem D-form")?;
    let frd = parse_reg(&ops[0], 'f')?;
    let (disp, ra, _) = parse_mem(&ops[1], symbols)?;
    Ok(d_form(opcd, frd, ra, disp as i32))
}
fn fa3(ops: &[String], opcd: u32, subop5: u32, rc: u32) -> Result<u32, String> {
    need(ops, 3, "A-form 3-reg")?;
    let frd = parse_reg(&ops[0], 'f')?;
    let fra = parse_reg(&ops[1], 'f')?;
    let frb = parse_reg(&ops[2], 'f')?;
    Ok((opcd << 26) | (frd << 21) | (fra << 16) | (frb << 11) | (subop5 << 1) | rc)
}
fn fa_mul(ops: &[String], opcd: u32, subop5: u32, rc: u32) -> Result<u32, String> {
    // FRA, FRC layout for fmul/fmuls/ps_mul: f{rd}, f{ra}, f{rc}
    need(ops, 3, "A-form mul")?;
    let frd = parse_reg(&ops[0], 'f')?;
    let fra = parse_reg(&ops[1], 'f')?;
    let frc = parse_reg(&ops[2], 'f')?;
    Ok((opcd << 26) | (frd << 21) | (fra << 16) | (frc << 6) | (subop5 << 1) | rc)
}
fn fa4(ops: &[String], opcd: u32, subop5: u32, rc: u32) -> Result<u32, String> {
    need(ops, 4, "A-form 4-reg")?;
    let frd = parse_reg(&ops[0], 'f')?;
    let fra = parse_reg(&ops[1], 'f')?;
    let frc = parse_reg(&ops[2], 'f')?;
    let frb = parse_reg(&ops[3], 'f')?;
    Ok((opcd << 26) | (frd << 21) | (fra << 16) | (frb << 11) | (frc << 6) | (subop5 << 1) | rc)
}
fn fa_unary(ops: &[String], opcd: u32, subop5: u32, rc: u32) -> Result<u32, String> {
    need(ops, 2, "A-form unary")?;
    let frd = parse_reg(&ops[0], 'f')?;
    let frb = parse_reg(&ops[1], 'f')?;
    Ok((opcd << 26) | (frd << 21) | (frb << 11) | (subop5 << 1) | rc)
}
fn fx_unary(ops: &[String], opcd: u32, subop10: u32, rc: u32) -> Result<u32, String> {
    need(ops, 2, "X-form fp unary")?;
    let frd = parse_reg(&ops[0], 'f')?;
    let frb = parse_reg(&ops[1], 'f')?;
    Ok((opcd << 26) | (frd << 21) | (frb << 11) | (subop10 << 1) | rc)
}
fn ps_merge(ops: &[String], subop10: u32, rc: u32) -> Result<u32, String> {
    need(ops, 3, "ps_merge")?;
    let frd = parse_reg(&ops[0], 'f')?;
    let fra = parse_reg(&ops[1], 'f')?;
    let frb = parse_reg(&ops[2], 'f')?;
    Ok((4 << 26) | (frd << 21) | (fra << 16) | (frb << 11) | (subop10 << 1) | rc)
}
