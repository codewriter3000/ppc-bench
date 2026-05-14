# Instruction Reference

How each instruction *behaves*. Notation:

- `RA|0` — register `RA`, but reads as literal `0` when `RA = 0`.
- `sext(x)` — sign-extend `x` to 32 bits.
- `EA` — effective address.
- `MEM(EA, n)` — `n` consecutive bytes at `EA` (big-endian).
- `CR0 ← cmp(x)` — record-form update: set `LT/GT/EQ` from signed `x`.

## Integer Arithmetic

### `addi RT, RA, SI`
`RT ← (RA|0) + sext(SI)`. Used as `li RT, SI` when `RA=0`.

### `addis RT, RA, SI`
`RT ← (RA|0) + (sext(SI) << 16)`. Used as `lis RT, SI`.

### `addic RT, RA, SI` / `addic. RT, RA, SI`
Adds the sign-extended immediate to `RA`, **writes the carry bit** to
`XER.CA`. The dot form additionally updates `CR0`.

### `add RT, RA, RB` *(also `add.`, `addo`, `addo.`)*
`RT ← RA + RB`. The `o` form updates `XER.OV/SO` on signed overflow; the `.`
form updates `CR0`.

### `addc / adde / addze / addme`
- `addc` — `add` with carry-out into `XER.CA`.
- `adde` — `RT ← RA + RB + XER.CA`, also writes new `CA`.
- `addze` — `RT ← RA + XER.CA`.
- `addme` — `RT ← RA + XER.CA − 1`.

### `subf RT, RA, RB`
**Note the operand order**: `RT ← RB − RA`. The mnemonic `sub RT, RB, RA` is
an alias that re-orders the operands so the result reads naturally. The
`subfc/subfe/subfic/subfze/subfme` variants mirror the `addc`-family but with
the subtract-from semantics (the operand being subtracted is bit-inverted
plus carry).

### `neg RT, RA`
`RT ← −RA`. Equivalent to `subfic RT, RA, 0`.

### Multiply
- `mulli RT, RA, SI` — low 32 of signed `RA × sext(SI)`.
- `mullw RT, RA, RB` — low 32 of signed `RA × RB`.
- `mulhw / mulhwu` — high 32 of signed / unsigned product.

### Divide
- `divw RT, RA, RB` — signed truncating division. Division by zero or the
  overflow case (`INT_MIN / −1`) leaves `RT` unspecified and, with `OE=1`,
  sets `XER.OV/SO`.
- `divwu` — unsigned variant.

## Logical, Shift & Rotate

### Bitwise
The `and/or/xor/nand/nor/eqv/andc/orc` family operate as `RA ← RS op RB`.
The immediate forms `andi./andis./ori/oris/xori/xoris` use a zero-extended
16-bit immediate (or its `<<16` variant). Only the `andi.`/`andis.` immediates
update `CR0`.

### Sign extension & count-leading-zeros
- `extsb / extsh` — sign-extend the low 8 or 16 bits of `RS` into `RA`.
- `cntlzw RA, RS` — number of leading zero bits in `RS` (0..32).

### Shifts
- `slw RA, RS, RB` — shift left logical by `RB[26:31]` (low 6 bits). Counts
  ≥ 32 produce 0.
- `srw RA, RS, RB` — shift right logical (same count rules).
- `sraw RA, RS, RB` — shift right algebraic. Updates `XER.CA` if `RS < 0` and
  any 1-bit was shifted out.
- `srawi RA, RS, SH` — arithmetic shift right by immediate `SH` (0..31).

### Rotate-and-Mask (M-form)
The most flexible PPC instruction family. Mask defined by `MB..ME`, *cyclic*.

- `rlwinm RA, RS, SH, MB, ME` — rotate left by `SH`, mask by `[MB..ME]`, write.
- `rlwnm RA, RS, RB, MB, ME` — rotate amount in `RB[27:31]`.
- `rlwimi RA, RS, SH, MB, ME` — insert rotated bits into `RA` at masked positions.

Common idioms:

```
extlwi RA, RS, n, b   ≡ rlwinm RA, RS, b, 0, n-1
extrwi RA, RS, n, b   ≡ rlwinm RA, RS, b+n, 32-n, 31
slwi   RA, RS, n      ≡ rlwinm RA, RS, n, 0, 31-n
srwi   RA, RS, n      ≡ rlwinm RA, RS, 32-n, n, 31
clrlwi RA, RS, n      ≡ rlwinm RA, RS, 0, n, 31
clrrwi RA, RS, n      ≡ rlwinm RA, RS, 0, 0, 31-n
```

## Compare

`cmp BF, L, RA, RB` and `cmpi BF, L, RA, SI` write the result to `CR[BF]`:

```
LT bit ← RA < operand     (signed for cmp/cmpi, unsigned for cmpl/cmpli)
GT bit ← RA > operand
EQ bit ← RA = operand
SO bit ← XER.SO           (copied across)
```

The `L` field selects 32- vs 64-bit comparison; PPC-Bench treats `L=1` as
illegal (32-bit core).

## Branch

### `b LI [, AA, LK]`
PC-relative branch. `LI` is a 24-bit sign-extended displacement in word units
(actually `LI << 2`). With `AA=1` the target is absolute; with `LK=1` (`bl`)
the **next instruction address is written to `LR`** before branching.

### `bc BO, BI, BD`
Conditional branch on the bit `CR[BI]` (combined with the `BO` field that
controls CTR decrement, branch-if-true/false, prediction hint).

`BO` encoding (most-common cases):

| `BO`  | Behaviour                                                    |
| ----- | ------------------------------------------------------------ |
| `0b00100` (4)  | Branch if `CR[BI] = 0` (e.g. `bne`).                |
| `0b01100` (12) | Branch if `CR[BI] = 1` (e.g. `beq`).                |
| `0b10000` (16) | Decrement CTR, branch if `CTR ≠ 0` (e.g. `bdnz`).   |
| `0b10010` (18) | Decrement CTR, branch if `CTR = 0`.                 |
| `0b10100` (20) | Always branch (degenerates `bclr 20,0,0` → `blr`).  |

### `bclr` / `bcctr`
Same `BO/BI` decode as `bc`, but the target is `LR` or `CTR` (with low two
bits forced to 0). `bclr 20,0,0` ⇒ `blr`. `bcctr 20,0,0` ⇒ `bctr`.

## Condition Register Logic

The CR-logical ops treat the 32-bit CR as a flat array of bits:

```
CR[BT] ← CR[BA] {op} CR[BB]
```

`mcrf BF, BFA` copies a 4-bit CR field. `mfcr RT` copies the entire CR;
`mtcrf FXM, RS` writes selected fields under a mask `FXM`.

## System & SPR

- `mfspr RT, SPRn` / `mtspr SPRn, RS` — move between GPR and SPR. The
  encoded `SPRn` is the field-swapped 10-bit form `(spr[5:9] || spr[0:4])`.
  Mnemonics `mflr/mtlr/mfctr/mtctr/mfxer/mtxer` are aliases.
- `mfmsr / mtmsr` — read/write the Machine State Register.
- `sync / isync / eieio` — memory ordering barriers. Treated as NOPs by the
  emulator but accepted by the assembler/disassembler.
- `sc` — system call. Halts the emulator with `HaltReason::Trap`.
- `twi TO, RA, SI` / `tw TO, RA, RB` — trap word. Compares `RA` to the
  operand and traps if any condition in `TO` is met:
  - bit 0: signed `RA < op`
  - bit 1: signed `RA > op`
  - bit 2: `RA = op`
  - bit 3: unsigned `RA < op`
  - bit 4: unsigned `RA > op`

  `TO = 31` ⇒ unconditional trap (canonical `trap` mnemonic).

## Cache Instructions

`dcbz/dcbi/dcbf/dcbst/dcbt/dcbtst/icbi` are decoded and printed by the
disassembler but **do not modify state**. `dcbz` is not implemented because
PPC-Bench has no cache model; programs that rely on `dcbz` zeroing a cache
line will need to be patched.

## Load / Store

Computed effective address:

| Form      | EA                              | Update? |
| --------- | ------------------------------- | ------- |
| `lwz` (D)   | `EA = (RA|0) + sext(D)`       | No      |
| `lwzu` (D)  | `EA = RA + sext(D)`           | Yes (`RA ← EA`) |
| `lwzx` (X)  | `EA = (RA|0) + RB`            | No      |
| `lwzux` (X) | `EA = RA + RB`                | Yes (`RA ← EA`) |

For update forms `RA` must not be `0` and must not equal `RT` (architectural
rule). Byte-reverse variants (`lwbrx`/`stwbrx`/`lhbrx`/`sthbrx`) swap the
endianness of the access — useful for little-endian data interop.

`lmw RT, D(RA)` loads `r[RT]..r31` from consecutive words starting at EA;
`stmw RS, D(RA)` stores them. Both are used for prolog/epilog sequences.

## Trap / Halt Behaviour

When the emulator hits `sc`, `tw`, or any unimplemented instruction it stops
execution and reports the reason via the UI status pill and the
`HaltReason` field of the snapshot. See **CPU Architecture › Exception
Model** for the full list.
