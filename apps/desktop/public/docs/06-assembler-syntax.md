# Assembler Syntax

PPC-Bench ships with a two-pass text assembler tailored for *learning*: it
accepts a GNU-as-flavoured subset that covers all instructions implemented by
the interpreter. Source is assembled to a contiguous big-endian byte stream
beginning at `BASE_ADDR = 0x80000000`.

## Line Structure

```
[label:] [mnemonic operand1[, operand2[, …]]] [# comment]
```

- Labels: `[A-Za-z_][A-Za-z0-9_]*` followed by `:`. Resolved during pass 1.
- One instruction per line. Blank lines and `#`-comments are ignored.
- Whitespace between mnemonic and operands is arbitrary.

```ppc
# Compute r5 = 1 + 41, then return.
start:
    li      r3, 1
    li      r4, 41
    add     r5, r3, r4
    blr
```

## Operand Forms

### Registers

| Pattern | Range  | Notes                              |
| ------- | ------ | ---------------------------------- |
| `rN`    | 0–31   | General-purpose register.          |
| `fN`    | 0–31   | Floating-point register.           |
| `crN`   | 0–7    | Condition register field (4 bits). |

### Immediates

Accepted in any operand position:

- Decimal: `42`, `-1`
- Hex: `0x80000000`, `0xff`
- Binary: `0b1010`
- Symbol name (resolves to the symbol's address)

### Memory addressing

| Syntax            | Meaning                                |
| ----------------- | -------------------------------------- |
| `disp(rA)`        | D-form: `EA = (rA|0) + sext(disp)`     |
| `disp(rA, rB)`    | Indexed form (sugar) — same as `rA, rB`|
| `rA, rB`          | X-form: `EA = (rA|0) + rB`             |

The same syntax serves loads and stores; the mnemonic determines width and
signedness.

## Extended Mnemonics

The assembler recognises the conventional aliases:

| Extended    | Canonical encoding              | Description                       |
| ----------- | ------------------------------- | --------------------------------- |
| `li rT, SI` | `addi rT, 0, SI`                | Load immediate (sign-extended).   |
| `lis rT, SI`| `addis rT, 0, SI`               | Load immediate << 16.             |
| `mr rT, rA` | `or rT, rA, rA`                 | Register-to-register move.        |
| `nop`       | `ori 0, 0, 0`                   | No-op.                            |
| `not rT, rA`| `nor rT, rA, rA`                | Bitwise NOT.                      |
| `mtlr rS`   | `mtspr 8, rS`                   | Move to Link Register.            |
| `mflr rT`   | `mfspr rT, 8`                   | Move from Link Register.          |
| `mtctr rS`  | `mtspr 9, rS`                   | Move to Count Register.           |
| `mfctr rT`  | `mfspr rT, 9`                   | Move from Count Register.         |
| `mtxer rS`  | `mtspr 1, rS`                   | Move to XER.                      |
| `mfxer rT`  | `mfspr rT, 1`                   | Move from XER.                    |
| `blr`       | `bclr 20, 0, 0`                 | Return to LR.                     |
| `bctr`      | `bcctr 20, 0, 0`                | Branch to CTR.                    |
| `bdnz lbl`  | `bc 16, 0, lbl`                 | Decrement CTR, branch if ≠ 0.     |
| `beq lbl`   | `bc 12, 2, lbl`                 | Branch if `CR0.EQ`.               |
| `bne lbl`   | `bc 4, 2, lbl`                  | Branch if `!CR0.EQ`.              |
| `blt lbl`   | `bc 12, 0, lbl`                 | Branch if `CR0.LT`.               |
| `bgt lbl`   | `bc 12, 1, lbl`                 | Branch if `CR0.GT`.               |
| `ble lbl`   | `bc 4, 1, lbl`                  | Branch if `!CR0.GT`.              |
| `bge lbl`   | `bc 4, 0, lbl`                  | Branch if `!CR0.LT`.              |
| `trap`      | `tw 31, 0, 0`                   | Unconditional trap.               |

## Suffixes

The assembler accepts the standard record/overflow suffixes appended to a
core mnemonic:

- `add.` — record form, sets `CR0` (or `CR1` for FP).
- `addo` — enables overflow capture (`XER.OV/SO`).
- `addo.` — both.

These map onto the corresponding bit fields (`Rc`, `OE`) in the encoded word.

## Branch Targets

Branch instructions accept either a label (resolved to a 4-byte aligned
address) or an absolute immediate. The assembler computes a PC-relative
displacement automatically:

```ppc
loop:
    addi    r3, r3, 1
    cmpwi   r3, 10
    blt     loop          # → bc 12, 0, loop  (signed PC-rel)
```

> **Absolute branches** (`AA=1` form) and the per-instruction `LK` bit (`bl`)
> can be requested via the conventional `bla` / `bl` mnemonics where the
> assembler supports them; the interpreter executes both faithfully.

## Constants & Pseudo-ops

There is currently **no `.data` / `.byte` / `.long` directive set** — every
line must encode to a single 4-byte instruction. To embed data, place it
after a `.word`-equivalent `nop` chain or assemble it externally and load
the raw bytes via the engine's `loadProgram` API.

For background on `.data`, `.sdata`, and the EABI memory layout see
**§ 7 Data Sections & EABI Memory Map**.

## Error Reporting

Assembler errors appear in the **Code Editor** panel and in the status row:

- `unknown mnemonic 'xxx'` — typo or unsupported instruction.
- `bad operand …` — operand count or type mismatch.
- `unresolved symbol 'foo'` — branch target / label not defined.
- `displacement out of range` — branch beyond ±32 MiB (≈±8M instructions).

Each error includes the source-line number so it can be jumped to directly.
