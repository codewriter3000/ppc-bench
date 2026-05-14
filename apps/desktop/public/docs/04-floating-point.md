# Floating Point

PPC-Bench implements the PowerPC user-mode floating-point model: 32 × 64-bit
FPRs, an FPSCR, IEEE-754 arithmetic, and the Gekko paired-single extensions
(documented separately).

## Register Conventions (ABI)

| Register   | Convention      |
| ---------- | --------------- |
| `f0`       | Scratch         |
| `f1`–`f8`  | Argument / return |
| `f9`–`f13` | Volatile scratch |
| `f14`–`f31`| Saved (non-volatile) |

Each FPR is internally **64 bits** holding either:
- An IEEE-754 binary64 double, **or**
- Two binary32 singles (paired-single mode, see Gekko Extensions).

`lfs` / `stfs` automatically convert between the in-memory single-precision
representation and the in-register double.

## FPSCR

The Floating-Point Status & Control Register is a 32-bit register
combining sticky exception flags, current rounding mode, and exception
enables. Notable fields:

| Bits  | Field    | Meaning                                       |
| ----- | -------- | --------------------------------------------- |
| 0     | `FX`     | Exception summary (sticky OR of FI/FR/FU/etc.)|
| 1     | `FEX`    | Enabled exception summary                     |
| 2     | `VX`     | Invalid-operation summary                     |
| 3     | `OX`     | Overflow (sticky)                             |
| 4     | `UX`     | Underflow (sticky)                            |
| 5     | `ZX`     | Zero-divide (sticky)                          |
| 6     | `XX`     | Inexact (sticky)                              |
| 7–12  | `VXSNAN…VXCVI` | Specific invalid-op causes              |
| 13–14 | `FR`,`FI`| Fraction rounded / inexact                    |
| 15–19 | `FPRF`   | Result class (zero, normal, denorm, inf, NaN, signed) |
| 24–28 | `VE…XE`  | Exception enables                             |
| 30–31 | `RN`     | Rounding mode                                 |

### Rounding modes (`RN`)

| `RN` | Mode                              |
| ---- | --------------------------------- |
| `00` | Round to nearest (ties to even)   |
| `01` | Round toward zero (truncate)      |
| `10` | Round toward +∞                   |
| `11` | Round toward −∞                   |

Updated by `mtfsf` / `mtfsfi` / `mtfsb0` / `mtfsb1`. PPC-Bench performs all
arithmetic in IEEE-754 round-to-nearest and updates `FPRF`/`FI`/`FR` to match
the result class.

## Conversion Instructions

- `frsp FRT, FRB` — round the double in `FRB` to single precision, keep
  result as a 64-bit value in `FRT`.
- `fctiw FRT, FRB` — convert to signed 32-bit integer using current rounding
  mode; result placed in low 32 of `FRT` (high 32 undefined).
- `fctiwz FRT, FRB` — same, but **truncate** (round toward zero) regardless
  of `RN`.

For the integer→float direction, the canonical sequence is:

```
lis    r3, 0x4330
stw    r3, 0(sp)
xoris  r4, r4, 0x8000      ; flip sign bit
stw    r4, 4(sp)
lfd    f1, 0(sp)
lfd    f0, mci3(rtoc)      ; "magic constant" 2^52 + 2^31
fsub   f1, f1, f0
```

## Compare

- `fcmpu BF, FRA, FRB` — *unordered* compare. NaN inputs produce a "?" result
  without raising an invalid-operation exception (unless the `VE` enable is
  set).
- `fcmpo BF, FRA, FRB` — *ordered* compare. Raises `VXSNAN`/`VXVC` on NaNs.

Both write the 4-bit `CR[BF]` field with:

```
LT ← FRA < FRB
GT ← FRA > FRB
EQ ← FRA = FRB
SO ← unordered (either is NaN)
```

## Fused Multiply-Add

`fmadd/fmsub/fnmadd/fnmsub` and their `s`-precision variants perform the
multiplication and addition with **a single rounding** at the end. The
encoding allocates a third FPR `FRC` for the multiplier:

```
fmadd  FRT, FRA, FRC, FRB    →  FRT ← (FRA × FRC) + FRB
fmsub  FRT, FRA, FRC, FRB    →  FRT ← (FRA × FRC) − FRB
fnmadd FRT, FRA, FRC, FRB    →  FRT ← −((FRA × FRC) + FRB)
fnmsub FRT, FRA, FRC, FRB    →  FRT ← −((FRA × FRC) − FRB)
```

## Move / Sign Manipulation

- `fmr FRT, FRB` — copy.
- `fabs FRT, FRB` — clear sign bit.
- `fneg FRT, FRB` — toggle sign bit.
- `fnabs FRT, FRB` — set sign bit.

These do **not** raise IEEE exceptions even for NaNs/Infs; they are pure
bit-level operations on the sign bit.

## `fsel` — Branch-Free Select

`fsel FRT, FRA, FRC, FRB` writes `FRC` if `FRA ≥ 0.0`, else `FRB`. This is
used by compilers to implement saturating clamps without branches:

```
# clamp x to [0, 1]:
fsub   f2, f1, f0_one
fsel   f1, f2, f0_one, f1
fneg   f3, f1
fsel   f1, f3, f0_zero, f1
fneg   f1, f1
```

## Reciprocal Estimates

- `fres FRT, FRB` — `~ 1/FRB`, ≥12 bits of precision.
- `frsqrte FRT, FRB` — `~ 1/√FRB`, ≥12 bits of precision.

These are typically followed by one or two Newton-Raphson refinement steps
for full single-precision accuracy.

## Record Form for Floats

Floating-point instructions ending in `.` (e.g. `fadd.`) update **`CR1`**
(not `CR0`) with the high bits of FPSCR:

```
CR1 ← FX || FEX || VX || OX
```

## FPSCR Access

- `mffs FRT` — copy FPSCR to low 32 bits of `FRT` (high 32 undefined).
- `mtfsf FLM, FRB` — copy 4-bit fields of `FRB` to FPSCR under mask `FLM`.
- `mtfsfi BF, IMM` — write 4-bit immediate to `FPSCR[BF]`.
- `mtfsb0 / mtfsb1` — clear / set an individual FPSCR bit.

## Denormals & NaN Policy

PPC-Bench uses the host's IEEE-754 implementation: denormals are honoured,
quiet NaNs propagate, signalling NaNs are quieted when consumed. The
**non-IEEE mode** bit (FPSCR.NI) — which would flush denormals to zero —
is currently ignored.
