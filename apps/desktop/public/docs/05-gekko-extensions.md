# Gekko Extensions — Paired Singles

The **Gekko** (GameCube) and **Broadway** (Wii) CPUs extend PowerPC 750 with a
SIMD floating-point mode that treats every FPR as a pair of single-precision
values: `(ps0, ps1)`. This delivers 2× throughput for vectorisable workloads
such as 3D math and audio mixing.

## Paired-Single FPR Layout

```
 63                  32 31                   0
┌──────────────────────┬──────────────────────┐
│         ps0          │         ps1          │
│  single-precision    │  single-precision    │
└──────────────────────┴──────────────────────┘
```

When the CPU is in paired-single mode, **all** FP arithmetic operates lanewise.
PPC-Bench tracks both `ps0` and `ps1` per FPR in the `FPUSnapshot.fpr` array
exposed to the UI.

> **Compatibility:** Double-precision instructions (`fadd`, `fsub`, …) still
> operate on the full 64-bit value of `ps0`'s slot. They produce a result
> whose `ps1` half is *architecturally undefined*; the emulator preserves
> the previous lane to keep displays stable.

## Arithmetic

All `ps_*` arithmetic ops produce single-precision results in **both** lanes:

```
ps_add  FRT, FRA, FRB     →  ps0 ← FRA.ps0 + FRB.ps0
                             ps1 ← FRA.ps1 + FRB.ps1
```

The `ps_madd/msub/nmadd/nmsub` family does fused multiply-add per lane.

### Cross-lane variants

| Mnemonic     | `FRT.ps0`          | `FRT.ps1`          |
| ------------ | ------------------ | ------------------ |
| `ps_muls0`   | `FRA.ps0 × FRC.ps0`| `FRA.ps1 × FRC.ps0`|
| `ps_muls1`   | `FRA.ps0 × FRC.ps1`| `FRA.ps1 × FRC.ps1`|
| `ps_madds0`  | `(FRA.ps0×FRC.ps0)+FRB.ps0`| `(FRA.ps1×FRC.ps0)+FRB.ps1`|
| `ps_madds1`  | `(FRA.ps0×FRC.ps1)+FRB.ps0`| `(FRA.ps1×FRC.ps1)+FRB.ps1`|
| `ps_sum0`    | `FRA.ps0 + FRB.ps1`| `FRC.ps1`          |
| `ps_sum1`    | `FRC.ps0`          | `FRA.ps0 + FRB.ps1`|

`ps_sum0` and `ps_sum1` are the "horizontal add" primitives used by the
GameCube/Wii compilers to emit a 2-wide dot product:

```
# dot = a.ps0*b.ps0 + a.ps1*b.ps1, into f3.ps0:
ps_mul   f3, f1, f2          # f3 ← (a.ps0*b.ps0, a.ps1*b.ps1)
ps_sum0  f3, f3, f3, f3      # f3.ps0 ← f3.ps0 + f3.ps1
```

### Merge & swizzle

| Mnemonic     | `FRT.ps0` | `FRT.ps1` |
| ------------ | --------- | --------- |
| `ps_merge00` | `FRA.ps0` | `FRB.ps0` |
| `ps_merge01` | `FRA.ps0` | `FRB.ps1` |
| `ps_merge10` | `FRA.ps1` | `FRB.ps0` |
| `ps_merge11` | `FRA.ps1` | `FRB.ps1` |

`ps_merge00 fX, fY, fY` duplicates `ps0` into both lanes (broadcast).

### Sign / move

`ps_abs`, `ps_neg`, `ps_nabs`, `ps_mr` operate lanewise as bit-flips of the
sign bits — they never raise FP exceptions.

### `ps_sel` — branch-free lane select

```
FRT.psN ← (FRA.psN ≥ 0) ? FRC.psN : FRB.psN     (N = 0,1)
```

### Compare

`ps_cmpu0/ps_cmpo0` compare `ps0` lanes; `ps_cmpu1/ps_cmpo1` compare `ps1`
lanes. The result is written to a 4-bit `CR` field exactly like `fcmpu/fcmpo`.

## Quantized Load / Store (`psq_*`)

`psq_l` and `psq_st` are the headline Gekko instructions. They load or store
**two values in compressed form** and (de)quantize on the fly using one of
the 8 **Graphics Quantization Registers** (GQRs).

```
psq_l    FRT, D(RA), W, I    →  FRT ← dequantize_via_GQR[I](MEM(EA))
psq_st   FRS, D(RA), W, I    →  MEM(EA) ← quantize_via_GQR[I](FRS)
```

- `D` — 12-bit signed displacement (note: smaller than the 16-bit `lfs` disp).
- `W` — 1-bit "single quantize" flag (when 1, only `ps0` is processed).
- `I` — 3-bit GQR selector (0..7).

Indexed forms `psq_lx / psq_stx` use `RA + RB` instead of an immediate.
Update forms (`psq_lu / psq_stu / psq_lux / psq_stux`) write the EA back to
`RA` as with the integer loads.

## GQR (Graphics Quantization Register)

Each GQR is a 32-bit SPR (numbers 912–919) split into a *load* config and a
*store* config:

```
 31         24 23 22       19    15         8 7  6        3
┌─────────────┬──┬──────────┐  ┌─────────────┬──┬──────────┐
│   LD_SCALE  │  │  LD_TYPE │  │   ST_SCALE  │  │  ST_TYPE │
│   6 bits    │  │  3 bits  │  │   6 bits    │  │  3 bits  │
│  (signed)   │  │          │  │  (signed)   │  │          │
└─────────────┴──┴──────────┘  └─────────────┴──┴──────────┘
```

### Quantization types

| Code | Mnemonic | Element size  | Range / treatment              |
| ---- | -------- | ------------- | ------------------------------ |
| 0    | `float`  | 32 bits       | IEEE-754 single (scale ignored)|
| 4    | `u8`     | 8 bits        | Unsigned 0..255                |
| 5    | `u16`    | 16 bits       | Unsigned 0..65535              |
| 6    | `s8`     | 8 bits        | Signed −128..127               |
| 7    | `s16`    | 16 bits       | Signed −32768..32767           |

Codes 1–3 are reserved.

### Scale

`LD_SCALE` / `ST_SCALE` are 6-bit two's-complement values in `[−32, 31]`. The
scale is interpreted as a power of two:

```
loaded_float  = decoded_integer × 2^(−LD_SCALE)
stored_int    = round(input_float × 2^( ST_SCALE))
```

On load, the integer is read, sign- or zero-extended per the type, multiplied
by `2^(−LD_SCALE)`, and converted to single precision. On store, the float is
multiplied by `2^(ST_SCALE)`, rounded to integer, **saturated** to the target
range, and written.

### Setting a GQR

GQR `n` is SPR `912 + n` (`GQR0` is SPR 912). The mnemonic `mtspr 912, rX`
is preferred; some toolchains accept the alias `mtgqr0 rX`.

```
# Configure GQR0 for s16, scale = 15  (i.e. divide by 32768 on load):
lis    r3, 0x0007       # LD_TYPE = 7, ST_TYPE = 7 (both s16)
ori    r3, r3, 0x0707
oris   r3, r3, 0x0F0F   # scale = 15 in both fields
mtspr  912, r3
```

After this, `psq_l f1, 0(r4), 0, 0` reads two `s16` samples from memory and
expands them into normalized `[-1.0, 1.0)` paired-singles — exactly the
sequence used by GameCube/Wii audio code.

## Enabling Paired-Single Mode

The architectural enable is `HID2[PSE]` and `MSR[FP]`. PPC-Bench treats
paired-single instructions as **always enabled**; the emulator does not check
HID2 or MSR.FP before dispatch, because user code in the simulator is
expected to opt-in unconditionally.

## Practical Notes

- The `ps_*` mnemonics are **case-sensitive in lower** (the assembler accepts
  any case but the canonical mnemonic uses lower).
- The `W` bit in `psq_l/psq_st` is poorly named in some toolchains as `WX` —
  it is the same single-lane-quantize flag in either notation.
- When `W=1`, the unused `ps1` of the destination is **set to `1.0`** by
  `psq_l`, matching the hardware (useful for projection math).
