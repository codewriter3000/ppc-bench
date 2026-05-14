# CPU Architecture

PPC-Bench emulates the **PowerPC 750CL / Gekko / Broadway** core — a 32-bit
PowerPC processor used in the Nintendo GameCube and Wii. It implements the
PowerPC user instruction set (Book I), a subset of the supervisor model (Book
III), and the Gekko-specific **paired-single** floating-point extensions.

## Programming Model

The architecture is a classic RISC load/store design:

- **32 general-purpose registers** (GPRs), each 32-bit.
- **32 floating-point registers** (FPRs), each 64-bit and treated as either
  one double or **two paired singles** (`ps0`, `ps1`).
- A small bank of **special-purpose registers** (SPRs) — LR, CTR, XER, the
  Graphics Quantization Registers (GQRs), etc.
- A **Condition Register** (CR) with 8 four-bit fields (`CR0`..`CR7`).
- A **Floating-Point Status & Control Register** (FPSCR).
- A **Machine State Register** (MSR) describing the privileged execution mode.

## General-Purpose Registers

| Register   | Convention      | Notes                                   |
| ---------- | --------------- | --------------------------------------- |
| `r0`       | Scratch / zero  | Reads as `0` in some addressing forms.  |
| `r1`       | Stack pointer   | Quadword-aligned downward-growing stack.|
| `r2`       | TOC / `r13` pair| Small data pointer (per ABI).           |
| `r3`–`r10` | Argument / return regs                              ||
| `r11`–`r12`| Volatile scratch                                    ||
| `r13`–`r31`| Saved (non-volatile)                                ||

## Special-Purpose Registers

| SPR # | Name   | Purpose                                       |
| ----- | ------ | --------------------------------------------- |
| 1     | `XER`  | Carry/overflow/SO flags, byte count for lswx. |
| 8     | `LR`   | Link register (branch return address).        |
| 9     | `CTR`  | Counter register (loop counter, indirect br). |
| 912–919 | `GQR0`–`GQR7` | Gekko paired-single quantization control. |

`mfspr`, `mtspr`, `mflr`, `mtlr`, `mfctr`, `mtctr` move between GPRs and SPRs.

## Condition Register

The 32-bit CR is split into **eight 4-bit fields**. Each field holds:

```
 bit 0 = LT (negative)
 bit 1 = GT (positive)
 bit 2 = EQ (zero / equal)
 bit 3 = SO (summary overflow, copied from XER.SO)
```

- `CR0` is implicitly updated by *Record-form* instructions (mnemonics ending
  with `.`, e.g. `add.`, `subf.`).
- `CR1` is implicitly updated by record-form floating-point ops.
- `CR0`..`CR7` are explicitly named by compare and CR-logical instructions.

## XER Register

| Bit  | Field | Meaning                                  |
| ---- | ----- | ---------------------------------------- |
| 0    | `SO`  | Summary overflow (sticky).               |
| 1    | `OV`  | Overflow (set by `OE=1` arithmetic ops). |
| 2    | `CA`  | Carry out from `addc`, `adde`, `subfc` …  |
| 25–31| count | Byte count for `lswx`/`stswx`.           |

## Memory Model

- **Big-endian** by architectural default.
- **32-bit effective addresses**, no MMU translation in PPC-Bench.
- Loads/stores must be **naturally aligned** unless the mnemonic explicitly
  permits misalignment (the emulator allows unaligned access but reports
  alignment-sensitive instructions where applicable).
- All emulated code is loaded at `BASE_ADDR = 0x80000000`, mirroring the
  GameCube/Wii cached MEM1 region.

## Instruction Encoding Forms

Every PowerPC instruction is **exactly 4 bytes** and falls into one of these
encoding forms. The 6-bit **primary opcode** (`OPCD`) lives in bits 0–5.

| Form | Layout (bit fields)                              | Example       |
| ---- | ------------------------------------------------ | ------------- |
| **I**| `OPCD ¦ LI(24) ¦ AA ¦ LK`                        | `b`, `bl`     |
| **B**| `OPCD ¦ BO ¦ BI ¦ BD(14) ¦ AA ¦ LK`              | `bc`          |
| **D**| `OPCD ¦ RT ¦ RA ¦ D(16)`                         | `addi`, `lwz` |
| **DS** | `OPCD ¦ RT ¦ RA ¦ DS(14) ¦ XO(2)`              | `ld`, `std` (64-bit) |
| **X**| `OPCD ¦ RT ¦ RA ¦ RB ¦ XO(10) ¦ Rc`              | `add`, `lwzx` |
| **XO** | `OPCD ¦ RT ¦ RA ¦ RB ¦ OE ¦ XO(9) ¦ Rc`        | `addo.`       |
| **XL** | `OPCD ¦ BT ¦ BA ¦ BB ¦ XO(10) ¦ Rc`            | `crand`,`bclr`|
| **A**| `OPCD ¦ FRT ¦ FRA ¦ FRB ¦ FRC ¦ XO(5) ¦ Rc`      | `fmadd`       |
| **M**| `OPCD ¦ RS ¦ RA ¦ RB/SH ¦ MB ¦ ME ¦ Rc`          | `rlwinm`      |
| **MD/MDS** | 64-bit rotate forms                        | `rldicl`      |
| **SC** | `OPCD ¦ ... ¦ XO(1)`                           | `sc`          |

> **Tip:** Primary opcodes `19`, `31`, `59`, `63` are *extended* dispatchers.
> The decoder reads an additional 10-bit *XO* field to identify the real op.

## Pipeline & Timing

PPC-Bench is a functional simulator: it executes one instruction atomically
per `step()` and increments the global step counter. Cycle-accurate pipeline
modelling, branch prediction and cache effects are **not** simulated.

## Exception / Halt Model

The interpreter never traps to vectors. Instead, error conditions surface as
`HaltReason` values to the UI:

- `EndOfProgram` — PC reaches the program-end sentinel.
- `Trap` — `trap` / `tw` taken.
- `InvalidInstruction(opcode)` — Decoder failed.
- `MemoryError(msg)` — Out-of-range or unaligned access.
- `Breakpoint(addr)` — User breakpoint hit before fetch.
- `MaxStepsReached` — Cooperative budget exhausted (run-until).
