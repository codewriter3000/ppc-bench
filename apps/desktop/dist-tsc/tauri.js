/**
 * Typed wrappers around the Tauri `invoke` boundary. Argument names match the
 * Rust command signatures with Tauri's default camelCase→snake_case conversion.
 */
import { invoke } from "@tauri-apps/api/core";
export const api = {
    assemble: (source) => invoke("assemble", { source }),
    disassemble: (bytes, baseAddr) => invoke("disassemble", { bytes, baseAddr }),
    loadProgram: (bytes, symbols) => invoke("load_program", { bytes, symbols }),
    loadBinary: (bytes) => invoke("load_binary", { bytes }),
    step: () => invoke("step"),
    runUntil: (maxSteps) => invoke("run_until", { maxSteps }),
    launchWithEmulator: () => invoke("launch_with_emulator"),
    connectGdb: (port) => invoke("connect_gdb", { port }),
    captureEmulatorSnapshot: () => invoke("capture_emulator_snapshot"),
    emulatorContinue: () => invoke("emulator_continue"),
    emulatorStep: () => invoke("emulator_step"),
    emulatorBreak: () => invoke("emulator_break"),
    emulatorSetBreakpoint: (address) => invoke("emulator_set_breakpoint", { address }),
    emulatorClearBreakpoint: (address) => invoke("emulator_clear_breakpoint", { address }),
    emulatorReadMemory: (address, length) => invoke("emulator_read_memory", { address, length }),
    stopEmulator: () => invoke("stop_emulator"),
    probeEmulator: () => invoke("probe_emulator"),
    reset: () => invoke("reset"),
    getState: () => invoke("get_state"),
    setBreakpoint: (address) => invoke("set_breakpoint", { address }),
    clearBreakpoint: (address) => invoke("clear_breakpoint", { address }),
    getBreakpoints: () => invoke("get_breakpoints"),
    readMemory: (address, length) => invoke("read_memory", { address, length }),
    getTrace: (max) => invoke("get_trace", { max }),
    getSymbols: () => invoke("get_symbols"),
};
