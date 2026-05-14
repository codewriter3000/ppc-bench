# Data Sections & the EABI Memory Map

PowerPC programs produced by GCC/Metrowerks follow the **Embedded Application
Binary Interface (EABI)** memory layout. Understanding the named sections is
essential for reading linker maps, disassembly output, and memory dumps.

---

## Section Overview

| Section   | Permissions | Typical content                              |
| --------- | ----------- | -------------------------------------------- |
| `.text`   | r-x         | Executable code.                             |
| `.rodata` | r--         | String literals, `const` arrays, vtables.    |
| `.data`   | rw-         | Explicitly initialised global/static data.   |
| `.sdata`  | rw-         | Small initialised globals (≤ 8 bytes each).  |
| `.sdata2` | r--         | Small read-only data (EABI SDA2).            |
| `.bss`    | rw-         | Zero-initialised globals (no file bytes).    |
| `.sbss`   | rw-         | Small zero-initialised globals.              |

The linker packs all sections into the raw binary. On GameCube/Wii, the
runtime startup stub (`__start`) copies `.data` from ROM into RAM, zeros
`.bss`/`.sbss`, and sets up the two Small Data Area base registers before
calling `main`.

---

## `.data` — Initialised Read-Write Data

```ppc
# C equivalent:
#   int counter = 42;
#   float scale = 1.5f;

.data
counter:  .long  42          # 4-byte big-endian word
scale:    .float 1.5         # 4-byte IEEE-754
msg:      .string "hello"    # null-terminated UTF-8
```

- Every byte is stored verbatim in the ELF/DOL image and copied to RAM at
  load time.
- Accesses use a base register plus a signed 16-bit displacement:

```ppc
    lis     r4, counter@ha      # r4  = upper 16 bits of &counter
    lwz     r3, counter@l(r4)   # r3  = counter (full 32-bit address)
```

The `@ha` / `@l` relocations split the 32-bit address across two instructions
(`addis` + load/store). GCC emits this pattern for every non-small global.

---

## `.sdata` — Small Data Area (SDA1, base register `r13`)

The EABI reserves `r13` as the **Small Data Area (SDA) pointer** at runtime,
pointing to the middle of a ±32 KiB window that spans `.sdata` and `.sbss`.

```ppc
# C equivalent:
#   int x = 7;              // placed in .sdata  (≤ 8 bytes → compiler heuristic)
#   int buf[1024] = {0};    // placed in .bss    (too large for SDA)

.sdata
x:  .long 7
```

Because `r13` already holds the base, a small global needs only **one
instruction** to access instead of two:

```ppc
    lwz     r3, x@sda21(r13)    # r3 = x  (SDA-relative, 21-bit displacement)
```

The `@sda21` relocation tells the linker to compute the signed offset from the
SDA base and encode it in the instruction's immediate field. If the offset
falls outside ±32 KiB the linker rejects it, so large or many-variable
programs may overflow the SDA.

### SDA layout

```
    .sbss  (zero-init small globals)
           ↑
    r13 ───┤  SDA base (mid-point of the ±32 KiB window)
           ↓
    .sdata (initialised small globals)
```

The base address of `r13` is chosen by the linker so that both `.sdata` and
`.sbss` fit within the signed 16-bit offset range from `r13`.

---

## `.sdata2` — Small Read-Only Data Area (SDA2, base register `r2`)

`r2` is the **SDA2 pointer**, pointing into the small read-only window that
spans `.sdata2` and `.sbss2`.

```ppc
.sdata2
pi_approx:  .float 3.14159
```

Access:

```ppc
    lfs     f1, pi_approx@sda21(r2)   # f1 = pi_approx (SDA2-relative)
```

On GameCube/Wii, `r2` is initialised by the SDK startup code before `main`.
The compiler places `const` floating-point literals and other small read-only
values here to avoid the two-instruction `@ha`/`@l` sequence.

---

## Comparison: `@ha`/`@l` vs `@sda21`

| Situation                           | Code sequence   | Instructions |
| ----------------------------------- | --------------- | ------------ |
| Global in `.data` (any size)        | `lis`/`lwz`     | 2            |
| Small global in `.sdata` (≤ 8 B)   | `lwz @sda21`    | 1            |
| Small `const` in `.sdata2` (≤ 8 B) | `lfs @sda21`    | 1            |

The one-instruction form saves a register allocation slot and is faster on
in-order cores like the Gekko, which is why the compilers prefer it
aggressively for globals that fit the size threshold.

---

## How Globals Appear in PPC-Bench Disassembly

The PPC-Bench disassembler does not apply relocations, so you will see the
raw encoded immediates. Typical patterns to recognise:

### Two-instruction absolute load

```
80003400  3c 60 80 00   lis   r3, -32768        # @ha: upper half of 0x80000000
80003404  80 63 01 00   lwz   r3, 256(r3)       # @l:  lower 16 bits = 0x0100
                                                # → loads from 0x80000100
```

### SDA-relative load

```
80003410  80 6d ff f8   lwz   r3, -8(r13)       # r13 + (-8) = SDA global
```

The displacement is signed and 16 bits. If `r13` is known (it is a callee-
saved register and normally constant throughout a function), you can resolve
the address by reading `r13` from the Registers panel and adding the offset.

---

## PPC-Bench Assembler Limitations

The built-in assembler currently only handles the `.text` (code) section.
The directives below are **not** supported and will produce an error:

`.data`, `.sdata`, `.sdata2`, `.rodata`, `.bss`, `.sbss`,
`.byte`, `.short`, `.long`, `.float`, `.double`, `.string`, `.asciz`

To work with initialised data in PPC-Bench today:

1. **Encode data as instructions** — `nop` (`ori 0,0,0`) is `0x60000000`;
   you can chain arbitrary 32-bit words using `tw 0, rA, rB` encodings that
   will never trap, though this is awkward.
2. **Use the `loadProgram` API** — assemble the code externally (e.g.
   `powerpc-eabi-as`), then feed the raw byte array to the engine's
   `loadProgram` command. The memory panel can inspect the resulting layout.
3. **Write to memory at runtime** — use `lis`/`li`/`stw` sequences to plant
   data values before the code that reads them.
