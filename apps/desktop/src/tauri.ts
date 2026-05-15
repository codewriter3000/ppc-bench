/**
 * Typed wrappers around the Tauri `invoke` boundary. Argument names match the
 * Rust command signatures with Tauri's default camelCase→snake_case conversion.
 */
import { invoke } from "@tauri-apps/api/core";
import type {
  AssembleResult,
  BinaryLoadResult,
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

export interface EmulatorLaunchResult {
  dol_path: string;
  dolphin_path: string;
  gdb_port: number;
}

export type EmulatorWatchpointKind = "Write" | "Read" | "Access";

export interface EmulatorSignalStop {
  signal: number;
  exception_code: string | null;
}

export interface EmulatorWatchpointStop extends EmulatorSignalStop {
  kind: EmulatorWatchpointKind;
  address: number;
}

export type EmulatorStopReason =
  | "InitialStop"
  | "Step"
  | "ManualBreak"
  | { Breakpoint: number }
  | { Watchpoint: EmulatorWatchpointStop }
  | { Signal: EmulatorSignalStop };

export interface EmulatorStopResult {
  snapshot: MachineStateSnapshot;
  reason: EmulatorStopReason;
}

export const api = {
  assemble: (source: string) => invoke<AssembleResult>("assemble", { source }),
  disassemble: (bytes: number[], baseAddr: number) =>
    invoke<DisasmLine[]>("disassemble", { bytes, baseAddr }),
  loadProgram: (bytes: number[], symbols: ReadonlyArray<readonly [string, number]>) =>
    invoke<MachineStateSnapshot>("load_program", { bytes, symbols }),
  loadBinary: (bytes: number[]) => invoke<BinaryLoadResult>("load_binary", { bytes }),
  step: () => invoke<StepResult>("step"),
  runUntil: (maxSteps: number) => invoke<RunResult>("run_until", { maxSteps }),
  launchWithEmulator: () => invoke<EmulatorLaunchResult>("launch_with_emulator"),
  connectGdb: (port: number) => invoke<EmulatorStopResult>("connect_gdb", { port }),
  captureEmulatorSnapshot: () => invoke<MachineStateSnapshot>("capture_emulator_snapshot"),
  emulatorContinue: () => invoke<void>("emulator_continue"),
  emulatorStep: () => invoke<EmulatorStopResult>("emulator_step"),
  emulatorBreak: () => invoke<void>("emulator_break"),
  emulatorSetBreakpoint: (address: number) =>
    invoke<number[]>("emulator_set_breakpoint", { address }),
  emulatorClearBreakpoint: (address: number) =>
    invoke<number[]>("emulator_clear_breakpoint", { address }),
  emulatorReadMemory: (address: number, length: number) =>
    invoke<number[]>("emulator_read_memory", { address, length }),
  stopEmulator: () => invoke<void>("stop_emulator"),
  probeEmulator: () => invoke<string | null>("probe_emulator"),
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
