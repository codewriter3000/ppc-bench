import type { AssembleResult, BinaryLoadResult, DisasmLine, HaltReason, MachineStateSnapshot, StepResult, TraceEntry } from "@ppc-bench/kernel";
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
export type EmulatorStopReason = "InitialStop" | "Step" | "ManualBreak" | {
    Breakpoint: number;
} | {
    Watchpoint: EmulatorWatchpointStop;
} | {
    Signal: EmulatorSignalStop;
};
export interface EmulatorStopResult {
    snapshot: MachineStateSnapshot;
    reason: EmulatorStopReason;
}
export declare const api: {
    readonly assemble: (source: string) => Promise<AssembleResult>;
    readonly disassemble: (bytes: number[], baseAddr: number) => Promise<DisasmLine[]>;
    readonly loadProgram: (bytes: number[], symbols: ReadonlyArray<readonly [string, number]>) => Promise<MachineStateSnapshot>;
    readonly loadBinary: (bytes: number[]) => Promise<BinaryLoadResult>;
    readonly step: () => Promise<StepResult>;
    readonly runUntil: (maxSteps: number) => Promise<RunResult>;
    readonly launchWithEmulator: () => Promise<EmulatorLaunchResult>;
    readonly connectGdb: (port: number) => Promise<EmulatorStopResult>;
    readonly captureEmulatorSnapshot: () => Promise<MachineStateSnapshot>;
    readonly emulatorContinue: () => Promise<void>;
    readonly emulatorStep: () => Promise<EmulatorStopResult>;
    readonly emulatorBreak: () => Promise<void>;
    readonly emulatorSetBreakpoint: (address: number) => Promise<number[]>;
    readonly emulatorClearBreakpoint: (address: number) => Promise<number[]>;
    readonly emulatorReadMemory: (address: number, length: number) => Promise<number[]>;
    readonly stopEmulator: () => Promise<void>;
    readonly probeEmulator: () => Promise<string | null>;
    readonly reset: () => Promise<MachineStateSnapshot>;
    readonly getState: () => Promise<MachineStateSnapshot>;
    readonly setBreakpoint: (address: number) => Promise<number[]>;
    readonly clearBreakpoint: (address: number) => Promise<number[]>;
    readonly getBreakpoints: () => Promise<number[]>;
    readonly readMemory: (address: number, length: number) => Promise<number[]>;
    readonly getTrace: (max: number) => Promise<TraceEntry[]>;
    readonly getSymbols: () => Promise<[string, number][]>;
};
export type Api = typeof api;
