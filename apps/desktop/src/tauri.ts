/**
 * Typed wrappers around the Tauri `invoke` boundary. Argument names match the
 * Rust command signatures with Tauri's default camelCase→snake_case conversion.
 */
import { invoke } from "@tauri-apps/api/core";
import type {
  AssembleResult,
  DisasmLine,
  HaltReason,
  MachineStateSnapshot,
  StepResult,
  TraceEntry,
} from "@ppc-bench/kernel";

export interface RunResult {
  state: MachineStateSnapshot;
  steps_executed: number;
  halt_reason: HaltReason;
}

export const api = {
  assemble: (source: string) => invoke<AssembleResult>("assemble", { source }),
  disassemble: (bytes: number[], baseAddr: number) =>
    invoke<DisasmLine[]>("disassemble", { bytes, baseAddr }),
  loadProgram: (bytes: number[], symbols: ReadonlyArray<readonly [string, number]>) =>
    invoke<MachineStateSnapshot>("load_program", { bytes, symbols }),
  step: () => invoke<StepResult>("step"),
  runUntil: (maxSteps: number) => invoke<RunResult>("run_until", { maxSteps }),
  reset: () => invoke<MachineStateSnapshot>("reset"),
  getState: () => invoke<MachineStateSnapshot>("get_state"),
  setBreakpoint: (address: number) => invoke<number[]>("set_breakpoint", { address }),
  clearBreakpoint: (address: number) => invoke<number[]>("clear_breakpoint", { address }),
  getBreakpoints: () => invoke<number[]>("get_breakpoints"),
  readMemory: (address: number, length: number) =>
    invoke<number[]>("read_memory", { address, length }),
  getTrace: (max: number) => invoke<TraceEntry[]>("get_trace", { max }),
  getSymbols: () => invoke<Array<[string, number]>>("get_symbols"),
} as const;

export type Api = typeof api;
