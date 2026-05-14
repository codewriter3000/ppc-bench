/**
 * Shared type contracts between the Rust PPC engine (via Tauri commands) and
 * the SolidJS frontend. These must stay in sync with `apps/desktop/src-tauri/src/ppc_engine/`.
 */

/** 32 GPRs + PC + key SPRs. */
export interface RegisterSnapshot {
  gpr: readonly number[]; // length 32
  pc: number;
  lr: number;
  ctr: number;
  xer: number;
  cr: number;
  msr: number;
  /** Indices of GPRs that changed in the last step. */
  changed_gpr: readonly number[];
}

/** Paired-single FPRs: ps0 and ps1 per register. */
export interface FPUSnapshot {
  /** length 32, each entry is `[ps0, ps1]`. */
  fpr: readonly (readonly [number, number])[];
  fpscr: number;
  changed_fpr: readonly number[];
}

export interface CallFrame {
  call_site: number;
  return_to: number;
  symbol?: string | null;
}

export interface TraceEntry {
  step: number;
  pc: number;
  raw: number;
  mnemonic: string;
  operands: string;
}

export interface DisasmLine {
  address: number;
  raw: number;
  mnemonic: string;
  operands: string;
  label?: string | null;
}

export interface AssembleError {
  line: number;
  message: string;
}

export interface AssembleResult {
  ok: boolean;
  bytes: number[];
  base_addr: number;
  symbols: ReadonlyArray<readonly [string, number]>;
  errors: AssembleError[];
}

export interface MemoryWrite {
  addr: number;
  size: number;
}

/** Serde-default representation of the Rust `HaltReason` enum. */
export type HaltReason =
  | "Running"
  | "EndOfProgram"
  | "Trap"
  | "MaxStepsReached"
  | { Breakpoint: number }
  | { InvalidInstruction: number }
  | { MemoryError: string };

export const haltReasonLabel = (h: HaltReason): string => {
  if (typeof h === "string") return h;
  if ("Breakpoint" in h)
    return `Breakpoint @ 0x${(h.Breakpoint >>> 0).toString(16).toUpperCase()}`;
  if ("InvalidInstruction" in h)
    return `Invalid 0x${(h.InvalidInstruction >>> 0).toString(16).toUpperCase()}`;
  if ("MemoryError" in h) return `MemError: ${h.MemoryError}`;
  return "Unknown";
};

export interface MachineStateSnapshot {
  registers: RegisterSnapshot;
  fpu: FPUSnapshot;
  step_count: number;
  halted: boolean;
  halt_reason: HaltReason;
  breakpoints: readonly number[];
  call_stack: readonly CallFrame[];
  symbols: ReadonlyArray<readonly [string, number]>;
  program_end: number;
  last_writes: readonly MemoryWrite[];
}

export interface StepResult {
  state: MachineStateSnapshot;
  trace: TraceEntry | null;
  error: string | null;
}

export interface RunResult {
  state: MachineStateSnapshot;
  steps_executed: number;
  halt_reason: HaltReason;
}
