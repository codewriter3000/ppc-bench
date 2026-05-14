import type {
  AssembleResult,
  DisasmLine,
  HaltReason,
  MachineStateSnapshot,
  StepResult,
  TraceEntry,
} from "./contracts";

/**
 * Discriminated event map for the kernel bus. UI panels subscribe to these
 * events to stay in sync with backend state.
 */
export type KernelEvents = {
  "step-complete": StepResult;
  "run-complete": { state: MachineStateSnapshot; steps_executed: number; halt_reason: HaltReason };
  "state-reset": MachineStateSnapshot;
  "state-loaded": MachineStateSnapshot;
  "assemble-complete": AssembleResult;
  "disasm-loaded": { lines: DisasmLine[]; base_addr: number };
  "breakpoint-toggle": { addr: number; enabled: boolean };
  "navigate-memory": { addr: number };
  "navigate-disasm": { addr: number };
  "trace-append": TraceEntry;
  "engine-error": { message: string };
};
