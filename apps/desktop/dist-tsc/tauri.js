/**
 * Typed wrappers around the Tauri `invoke` boundary. Argument names match the
 * Rust command signatures with Tauri's default camelCase→snake_case conversion.
 */
import { invoke } from "@tauri-apps/api/core";
export const api = {
    assemble: (source) => invoke("assemble", { source }),
    disassemble: (bytes, baseAddr) => invoke("disassemble", { bytes, baseAddr }),
    loadProgram: (bytes, symbols) => invoke("load_program", { bytes, symbols }),
    step: () => invoke("step"),
    runUntil: (maxSteps) => invoke("run_until", { maxSteps }),
    reset: () => invoke("reset"),
    getState: () => invoke("get_state"),
    setBreakpoint: (address) => invoke("set_breakpoint", { address }),
    clearBreakpoint: (address) => invoke("clear_breakpoint", { address }),
    getBreakpoints: () => invoke("get_breakpoints"),
    readMemory: (address, length) => invoke("read_memory", { address, length }),
    getTrace: (max) => invoke("get_trace", { max }),
    getSymbols: () => invoke("get_symbols"),
};
