import type { AssembleResult, DisasmLine, HaltReason, MachineStateSnapshot, StepResult, TraceEntry } from "@ppc-bench/kernel";
export interface RunResult {
    state: MachineStateSnapshot;
    steps_executed: number;
    halt_reason: HaltReason;
}
export declare const api: {
    readonly assemble: (source: string) => Promise<AssembleResult>;
    readonly disassemble: (bytes: number[], baseAddr: number) => Promise<DisasmLine[]>;
    readonly loadProgram: (bytes: number[], symbols: ReadonlyArray<readonly [string, number]>) => Promise<MachineStateSnapshot>;
    readonly step: () => Promise<StepResult>;
    readonly runUntil: (maxSteps: number) => Promise<RunResult>;
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
