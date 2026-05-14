# Instruction Table

Every instruction implemented by PPC-Bench, grouped by category. Columns:

- **Mnemonic** — assembler mnemonic (Rc/OE suffixes shown where applicable).
- **OPCD** — primary opcode (bits 0–5), decimal.
- **XO** — extended opcode (where applicable), decimal.
- **Form** — encoding form (see *CPU Architecture › Instruction Encoding Forms*).
- **Summary** — short description.

> Suffix conventions:
> - `.` = **Rc** form, updates `CR0` (or `CR1` for FP record-form).
> - `o` = **OE** form, updates `XER[SO,OV]`.
> - `u` = update form, writes EA back to `RA`.
> - `x` = indexed form (`RA+RB` instead of immediate).

## Integer Arithmetic

| Mnemonic    | OPCD | XO  | Form | Summary                                  |
| ----------- | ---- | --- | ---- | ---------------------------------------- |
| `addi`      | 14   | —   | D    | `RT ← (RA\|0) + sext(SI)`                |
| `addis`     | 15   | —   | D    | `RT ← (RA\|0) + (SI << 16)`              |
| `addic`     | 12   | —   | D    | `addi` + sets `XER.CA`                   |
| `addic.`    | 13   | —   | D    | `addic` + updates `CR0`                  |
| `add`       | 31   | 266 | XO   | `RT ← RA + RB`                           |
| `addo`      | 31   | 266 (OE=1) | XO | `add` + overflow check                |
| `addc`      | 31   | 10  | XO   | `add`, sets `XER.CA`                     |
| `adde`      | 31   | 138 | XO   | `RT ← RA + RB + XER.CA`                  |
| `addze`     | 31   | 202 | XO   | `RT ← RA + XER.CA`                       |
| `addme`     | 31   | 234 | XO   | `RT ← RA + XER.CA − 1`                   |
| `subf`      | 31   | 40  | XO   | `RT ← RB − RA`                           |
| `subfc`     | 31   | 8   | XO   | `subf`, sets `XER.CA`                    |
| `subfe`     | 31   | 136 | XO   | `RT ← ¬RA + RB + XER.CA`                 |
| `subfic`    | 8    | —   | D    | `RT ← sext(SI) − RA` + `XER.CA`          |
| `subfze`    | 31   | 200 | XO   | `RT ← ¬RA + XER.CA`                      |
| `subfme`    | 31   | 232 | XO   | `RT ← ¬RA + XER.CA − 1`                  |
| `neg`       | 31   | 104 | XO   | `RT ← −RA`                               |
| `mulli`     | 7    | —   | D    | `RT ← RA × sext(SI)` (low 32)            |
| `mullw`     | 31   | 235 | XO   | `RT ← (RA × RB)[32:63]`                  |
| `mulhw`     | 31   | 75  | XO   | Signed high 32 of `RA × RB`              |
| `mulhwu`    | 31   | 11  | XO   | Unsigned high 32 of `RA × RB`            |
| `divw`      | 31   | 491 | XO   | Signed 32-bit division                   |
| `divwu`     | 31   | 459 | XO   | Unsigned 32-bit division                 |

## Logical, Shift & Rotate

| Mnemonic | OPCD | XO  | Form | Summary                            |
| -------- | ---- | --- | ---- | ---------------------------------- |
| `and`    | 31   | 28  | X    | `RA ← RS & RB`                     |
| `or`     | 31   | 444 | X    | `RA ← RS \| RB`                    |
| `xor`    | 31   | 316 | X    | `RA ← RS ^ RB`                     |
| `nand`   | 31   | 476 | X    | `RA ← ¬(RS & RB)`                  |
| `nor`    | 31   | 124 | X    | `RA ← ¬(RS \| RB)`                 |
| `eqv`    | 31   | 284 | X    | `RA ← ¬(RS ^ RB)`                  |
| `andc`   | 31   | 60  | X    | `RA ← RS & ¬RB`                    |
| `orc`    | 31   | 412 | X    | `RA ← RS \| ¬RB`                   |
| `andi.`  | 28   | —   | D    | `RA ← RS & UI`, updates `CR0`      |
| `andis.` | 29   | —   | D    | `RA ← RS & (UI << 16)`, `CR0`      |
| `ori`    | 24   | —   | D    | `RA ← RS \| UI`                    |
| `oris`   | 25   | —   | D    | `RA ← RS \| (UI << 16)`            |
| `xori`   | 26   | —   | D    | `RA ← RS ^ UI`                     |
| `xoris`  | 27   | —   | D    | `RA ← RS ^ (UI << 16)`             |
| `extsb`  | 31   | 954 | X    | Sign-extend byte                   |
| `extsh`  | 31   | 922 | X    | Sign-extend halfword               |
| `cntlzw` | 31   | 26  | X    | Count leading zeros                |
| `slw`    | 31   | 24  | X    | Shift left logical                 |
| `srw`    | 31   | 536 | X    | Shift right logical                |
| `sraw`   | 31   | 792 | X    | Shift right algebraic              |
| `srawi`  | 31   | 824 | X    | `sraw` with immediate shift count  |
| `rlwinm` | 21   | —   | M    | Rotate-and-mask immediate          |
| `rlwnm`  | 23   | —   | M    | Rotate-and-mask via `RB`           |
| `rlwimi` | 20   | —   | M    | Rotate-and-mask insert             |

## Compare

| Mnemonic | OPCD | XO  | Form | Summary                          |
| -------- | ---- | --- | ---- | -------------------------------- |
| `cmp`    | 31   | 0   | X    | Signed `RA` vs `RB`              |
| `cmpi`   | 11   | —   | D    | Signed `RA` vs `sext(SI)`        |
| `cmpl`   | 31   | 32  | X    | Unsigned `RA` vs `RB`            |
| `cmpli`  | 10   | —   | D    | Unsigned `RA` vs `UI`            |

## Branch

| Mnemonic | OPCD | XO  | Form | Summary                                            |
| -------- | ---- | --- | ---- | -------------------------------------------------- |
| `b`      | 18   | —   | I    | Unconditional branch (`bl` if `LK=1`, `ba` if `AA=1`) |
| `bc`     | 16   | —   | B    | Conditional branch by `BO`/`BI`                    |
| `bclr`   | 19   | 16  | XL   | Branch to `LR` (used by `blr`)                     |
| `bcctr`  | 19   | 528 | XL   | Branch to `CTR` (used by `bctr`)                   |

### Common branch aliases

| Alias  | Encodes as      |
| ------ | --------------- |
| `bl`   | `b … LK=1`      |
| `blr`  | `bclr 20, 0, 0` |
| `bctr` | `bcctr 20, 0, 0`|
| `beq`  | `bc 12, CR.EQ`  |
| `bne`  | `bc  4, CR.EQ`  |
| `blt`  | `bc 12, CR.LT`  |
| `bgt`  | `bc 12, CR.GT`  |

## Condition Register Logic

| Mnemonic | OPCD | XO  | Form | Summary                       |
| -------- | ---- | --- | ---- | ----------------------------- |
| `mcrf`   | 19   | 0   | XL   | Move CR field                 |
| `crand`  | 19   | 257 | XL   | `CR[BT] ← CR[BA] & CR[BB]`    |
| `cror`   | 19   | 449 | XL   | `CR[BT] ← CR[BA] \| CR[BB]`   |
| `crxor`  | 19   | 193 | XL   | `CR[BT] ← CR[BA] ^ CR[BB]`    |
| `crnand` | 19   | 225 | XL   | `¬(CR[BA] & CR[BB])`          |
| `crnor`  | 19   | 33  | XL   | `¬(CR[BA] \| CR[BB])`         |
| `creqv`  | 19   | 289 | XL   | `¬(CR[BA] ^ CR[BB])`          |
| `crandc` | 19   | 129 | XL   | `CR[BA] & ¬CR[BB]`            |
| `crorc`  | 19   | 417 | XL   | `CR[BA] \| ¬CR[BB]`           |
| `mfcr`   | 31   | 19  | X    | `RT ← CR`                     |
| `mtcrf`  | 31   | 144 | X    | Move selected CR fields       |

## System / SPR

| Mnemonic | OPCD | XO  | Form | Summary                            |
| -------- | ---- | --- | ---- | ---------------------------------- |
| `mfspr`  | 31   | 339 | X    | `RT ← SPR[n]`                      |
| `mtspr`  | 31   | 467 | X    | `SPR[n] ← RS`                      |
| `mfmsr`  | 31   | 83  | X    | `RT ← MSR`                         |
| `mtmsr`  | 31   | 146 | X    | `MSR ← RS`                         |
| `sync`   | 31   | 598 | X    | Memory barrier (NOP in emulator)   |
| `isync`  | 19   | 150 | XL   | Instruction-fetch barrier (NOP)    |
| `eieio`  | 31   | 854 | X    | Enforce I/O ordering (NOP)         |
| `sc`     | 17   | —   | SC   | System call (halts emulator)       |
| `twi`    | 3    | —   | D    | Trap-word immediate                |
| `tw`     | 31   | 4   | X    | Trap word                          |

## Cache Hints (decoded, treated as NOPs)

| Mnemonic | OPCD | XO   | Form | Summary                       |
| -------- | ---- | ---- | ---- | ----------------------------- |
| `dcbz`   | 31   | 1014 | X    | Clear 32-byte cache block     |
| `dcbi`   | 31   | 470  | X    | Invalidate                    |
| `dcbf`   | 31   | 86   | X    | Flush                         |
| `dcbst`  | 31   | 54   | X    | Store                         |
| `dcbt`   | 31   | 278  | X    | Touch                         |
| `dcbtst` | 31   | 246  | X    | Touch for store               |
| `icbi`   | 31   | 982  | X    | Invalidate I-cache block      |

## Integer Load / Store

D-form variants take an immediate displacement `(disp)RA`; X-form (`x`) variants
use `RA+RB`; update (`u`) variants write the EA back to `RA`.

| Mnemonic | OPCD | XO  | Form | Bytes | Direction | Sign        |
| -------- | ---- | --- | ---- | ----- | --------- | ----------- |
| `lbz`    | 34   | —   | D    | 1     | Load      | Zero-extend |
| `lbzu`   | 35   | —   | D    | 1     | Load (update) | Zero    |
| `lbzx`   | 31   | 87  | X    | 1     | Load (X)  | Zero        |
| `lbzux`  | 31   | 119 | X    | 1     | Load (XU) | Zero        |
| `lhz`    | 40   | —   | D    | 2     | Load      | Zero        |
| `lhzu`   | 41   | —   | D    | 2     | Load (U)  | Zero        |
| `lhzx`   | 31   | 279 | X    | 2     | Load (X)  | Zero        |
| `lhzux`  | 31   | 311 | X    | 2     | Load (XU) | Zero        |
| `lha`    | 42   | —   | D    | 2     | Load      | Sign        |
| `lhau`   | 43   | —   | D    | 2     | Load (U)  | Sign        |
| `lhax`   | 31   | 343 | X    | 2     | Load (X)  | Sign        |
| `lhaux`  | 31   | 375 | X    | 2     | Load (XU) | Sign        |
| `lwz`    | 32   | —   | D    | 4     | Load      | —           |
| `lwzu`   | 33   | —   | D    | 4     | Load (U)  | —           |
| `lwzx`   | 31   | 23  | X    | 4     | Load (X)  | —           |
| `lwzux`  | 31   | 55  | X    | 4     | Load (XU) | —           |
| `stb`    | 38   | —   | D    | 1     | Store     | —           |
| `stbu`   | 39   | —   | D    | 1     | Store (U) | —           |
| `stbx`   | 31   | 215 | X    | 1     | Store (X) | —           |
| `stbux`  | 31   | 247 | X    | 1     | Store (XU)| —           |
| `sth`    | 44   | —   | D    | 2     | Store     | —           |
| `sthu`   | 45   | —   | D    | 2     | Store (U) | —           |
| `sthx`   | 31   | 407 | X    | 2     | Store (X) | —           |
| `sthux`  | 31   | 439 | X    | 2     | Store (XU)| —           |
| `stw`    | 36   | —   | D    | 4     | Store     | —           |
| `stwu`   | 37   | —   | D    | 4     | Store (U) | —           |
| `stwx`   | 31   | 151 | X    | 4     | Store (X) | —           |
| `stwux`  | 31   | 183 | X    | 4     | Store (XU)| —           |
| `lmw`    | 46   | —   | D    | 4×N   | Load multiple | —      |
| `stmw`   | 47   | —   | D    | 4×N   | Store multiple| —      |
| `lwbrx`  | 31   | 534 | X    | 4     | Load byte-reversed | —  |
| `stwbrx` | 31   | 662 | X    | 4     | Store byte-reversed | — |
| `lhbrx`  | 31   | 790 | X    | 2     | Load halfword reversed | — |
| `sthbrx` | 31   | 918 | X    | 2     | Store halfword reversed | — |

## Floating-Point Load / Store

| Mnemonic | OPCD | XO  | Form | Bytes | Notes                       |
| -------- | ---- | --- | ---- | ----- | --------------------------- |
| `lfs`    | 48   | —   | D    | 4     | Load single → double in FRT |
| `lfsu`   | 49   | —   | D    | 4     | Load single (update)        |
| `lfsx`   | 31   | 535 | X    | 4     | Load single (indexed)       |
| `lfsux`  | 31   | 567 | X    | 4     | Load single (X+update)      |
| `lfd`    | 50   | —   | D    | 8     | Load double                 |
| `lfdu`   | 51   | —   | D    | 8     | Load double (update)        |
| `lfdx`   | 31   | 599 | X    | 8     | Load double (indexed)       |
| `lfdux`  | 31   | 631 | X    | 8     | Load double (X+update)      |
| `stfs`   | 52   | —   | D    | 4     | Convert double → single, store |
| `stfsu`  | 53   | —   | D    | 4     | Store single (update)       |
| `stfsx`  | 31   | 663 | X    | 4     | Store single (indexed)      |
| `stfsux` | 31   | 695 | X    | 4     | Store single (X+update)     |
| `stfd`   | 54   | —   | D    | 8     | Store double                |
| `stfdu`  | 55   | —   | D    | 8     | Store double (update)       |
| `stfdx`  | 31   | 727 | X    | 8     | Store double (indexed)      |
| `stfdux` | 31   | 759 | X    | 8     | Store double (X+update)     |

## Floating-Point Arithmetic — Single (OPCD 59)

| Mnemonic   | XO | Form | Summary                              |
| ---------- | -- | ---- | ------------------------------------ |
| `fadds`    | 21 | A    | `FRT ← single(FRA + FRB)`            |
| `fsubs`    | 20 | A    | `FRT ← single(FRA − FRB)`            |
| `fmuls`    | 25 | A    | `FRT ← single(FRA × FRC)`            |
| `fdivs`    | 18 | A    | `FRT ← single(FRA ÷ FRB)`            |
| `fmadds`   | 29 | A    | `FRT ← single((FRA × FRC) + FRB)`    |
| `fmsubs`   | 28 | A    | `FRT ← single((FRA × FRC) − FRB)`    |
| `fnmadds`  | 31 | A    | `FRT ← single(−((FRA × FRC) + FRB))` |
| `fnmsubs`  | 30 | A    | `FRT ← single(−((FRA × FRC) − FRB))` |
| `fres`     | 24 | A    | Reciprocal estimate (single)         |
| `frsqrte`  | 26 | A    | Reciprocal square-root estimate      |

## Floating-Point Arithmetic — Double (OPCD 63)

| Mnemonic | XO  | Form | Summary                          |
| -------- | --- | ---- | -------------------------------- |
| `fadd`   | 21  | A    | `FRT ← FRA + FRB`                |
| `fsub`   | 20  | A    | `FRT ← FRA − FRB`                |
| `fmul`   | 25  | A    | `FRT ← FRA × FRC`                |
| `fdiv`   | 18  | A    | `FRT ← FRA ÷ FRB`                |
| `fmadd`  | 29  | A    | `FRT ← (FRA × FRC) + FRB`        |
| `fmsub`  | 28  | A    | `FRT ← (FRA × FRC) − FRB`        |
| `fnmadd` | 31  | A    | `FRT ← −((FRA × FRC) + FRB)`     |
| `fnmsub` | 30  | A    | `FRT ← −((FRA × FRC) − FRB)`     |
| `fsqrt`  | 22  | A    | `FRT ← √FRB`                     |
| `frsp`   | 12  | X    | Round double → single (in 64-bit)|
| `fabs`   | 264 | X    | Absolute value                   |
| `fneg`   | 40  | X    | Negate                           |
| `fnabs`  | 136 | X    | Negative absolute                |
| `fmr`    | 72  | X    | Move FPR                         |
| `fsel`   | 23  | A    | `FRT ← (FRA ≥ 0) ? FRC : FRB`    |
| `fctiw`  | 14  | X    | Convert to int word              |
| `fctiwz` | 15  | X    | Convert to int word (truncate)   |
| `fcmpu`  | 0   | X    | Unordered compare                |
| `fcmpo`  | 32  | X    | Ordered compare                  |
| `mtfsf`  | 711 | XFL  | Move to FPSCR fields             |
| `mffs`   | 583 | X    | Move from FPSCR                  |

## Paired-Single (Gekko, OPCD 4 / 56–63)

| Mnemonic     | OPCD | XO  | Summary                                                |
| ------------ | ---- | --- | ------------------------------------------------------ |
| `ps_add`     | 4    | 21  | `(ps0,ps1) ← FRA + FRB` (lanewise, single precision)   |
| `ps_sub`     | 4    | 20  | Lanewise subtract                                      |
| `ps_mul`     | 4    | 25  | Lanewise multiply                                      |
| `ps_div`     | 4    | 18  | Lanewise divide                                        |
| `ps_madd`    | 4    | 29  | Lanewise `FRA×FRC + FRB`                               |
| `ps_msub`    | 4    | 28  | Lanewise `FRA×FRC − FRB`                               |
| `ps_nmadd`   | 4    | 31  | Negated lanewise MADD                                  |
| `ps_nmsub`   | 4    | 30  | Negated lanewise MSUB                                  |
| `ps_muls0`   | 4    | 12  | Broadcast `FRC.ps0` then multiply                      |
| `ps_muls1`   | 4    | 13  | Broadcast `FRC.ps1` then multiply                      |
| `ps_madds0`  | 4    | 14  | `muls0` + add                                          |
| `ps_madds1`  | 4    | 15  | `muls1` + add                                          |
| `ps_sum0`    | 4    | 10  | `(FRA.ps0+FRB.ps1, FRC.ps1)`                           |
| `ps_sum1`    | 4    | 11  | `(FRC.ps0, FRA.ps0+FRB.ps1)`                           |
| `ps_merge00` | 4    | 528 | `(FRA.ps0, FRB.ps0)`                                   |
| `ps_merge01` | 4    | 560 | `(FRA.ps0, FRB.ps1)`                                   |
| `ps_merge10` | 4    | 592 | `(FRA.ps1, FRB.ps0)`                                   |
| `ps_merge11` | 4    | 624 | `(FRA.ps1, FRB.ps1)`                                   |
| `ps_abs`     | 4    | 264 | Lanewise `|x|`                                         |
| `ps_neg`     | 4    | 40  | Lanewise `−x`                                          |
| `ps_nabs`    | 4    | 136 | Lanewise `−|x|`                                        |
| `ps_mr`      | 4    | 72  | Copy paired-single                                     |
| `ps_res`     | 4    | 24  | Lanewise reciprocal estimate                           |
| `ps_rsqrte`  | 4    | 26  | Lanewise reciprocal sqrt estimate                      |
| `ps_sel`     | 4    | 23  | Lanewise `(FRA ≥ 0) ? FRC : FRB`                       |
| `ps_cmpu0`   | 4    | 0   | Unordered compare of `ps0` lanes                       |
| `ps_cmpu1`   | 4    | 64  | Unordered compare of `ps1` lanes                       |
| `ps_cmpo0`   | 4    | 32  | Ordered compare of `ps0` lanes                         |
| `ps_cmpo1`   | 4    | 96  | Ordered compare of `ps1` lanes                         |
| `psq_l`      | 56   | —   | Load paired-single (dequantized via `GQR[I]`)          |
| `psq_lu`     | 57   | —   | Load paired-single (update)                            |
| `psq_lx`     | 4    | 6   | Load paired-single (indexed)                           |
| `psq_lux`    | 4    | 38  | Load paired-single (indexed, update)                   |
| `psq_st`     | 60   | —   | Store paired-single (quantized via `GQR[I]`)           |
| `psq_stu`    | 61   | —   | Store paired-single (update)                           |
| `psq_stx`    | 4    | 7   | Store paired-single (indexed)                          |
| `psq_stux`   | 4    | 39  | Store paired-single (indexed, update)                  |
