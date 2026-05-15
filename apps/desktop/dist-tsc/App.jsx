import { listen } from "@tauri-apps/api/event";
import { createSignal, createMemo, createEffect, onCleanup, onMount } from "solid-js";
import { createStore, produce } from "solid-js/store";
import { BreakpointsPanel, CallStackPanel, CodeEditorPanel, ControlBar, DiagnosticsPanel, DisassemblyPanel, FPUPanel, MemoryPanel, PPCBenchShell, RegistersPanel, ResizableColumn, SnapshotDiffPanel, SymbolTablePanel, } from "@ppc-bench/ui";
import { SnapshotHistory, haltReasonLabel as formatHaltReasonLabel } from "@ppc-bench/kernel";
import { DesktopTopbar } from "./DesktopTopbar";
import { DEFAULT_DESKTOP_SETTINGS, loadDesktopSettings, listenForDesktopSettingsUpdates } from "./desktopSettings";
import { openSettingsWindow } from "./openSettings";
import { api } from "./tauri";
import { openManualsWindow } from "./openManuals";
const BASE_ADDR = 0x8000_0000 >>> 0;
const RAM_SIZE = 16 * 1024 * 1024;
const RAM_END = (BASE_ADDR + RAM_SIZE) >>> 0;
const STACK_WINDOW_BYTES = 4096;
const DEFAULT_SOURCE = `# PPC-Bench — sample program
# Computes r5 = 1 + 41 = 42, then returns.
start:
    li      r3, 1
    li      r4, 41
    add     r5, r3, r4
    blr
`;
const EMPTY_STATE = {
    registers: {
        gpr: Array(32).fill(0),
        pc: BASE_ADDR,
        lr: 0,
        ctr: 0,
        xer: 0,
        cr: 0,
        msr: 0,
        changed_gpr: [],
        gqr: Array(8).fill(0),
    },
    fpu: { fpr: Array(32).fill([0, 0]), fpscr: 0, changed_fpr: [] },
    step_count: 0,
    halted: false,
    halt_reason: "Running",
    breakpoints: [],
    call_stack: [],
    symbols: [],
    program_end: BASE_ADDR,
    last_writes: [],
};
const hex32 = (n) => "0x" + ("0000000" + (n >>> 0).toString(16).toUpperCase()).slice(-8);
const BREAKPOINT_CONDITION_RE = /^\s*([A-Za-z][A-Za-z0-9_]*)\s*(==|!=|<=|>=|<|>)\s*([A-Za-z][A-Za-z0-9_]*|0x[0-9a-fA-F]+|\d+)\s*$/;
const ERROR_ADDRESS_RE = /0x([0-9a-fA-F]+)/;
const nowMs = () => performance.now();
const parseBreakpointCondition = (input) => {
    const match = BREAKPOINT_CONDITION_RE.exec(input);
    if (!match) {
        return null;
    }
    return {
        left: match[1] ?? "",
        operator: (match[2] ?? "=="),
        right: match[3] ?? "",
    };
};
const resolveBreakpointOperand = (snapshot, token) => {
    const normalized = token.trim().toLowerCase();
    const registerMatch = /^r([0-9]|[12][0-9]|3[01])$/.exec(normalized);
    if (registerMatch) {
        return (snapshot.registers.gpr[Number(registerMatch[1])] ?? 0) >>> 0;
    }
    if (normalized === "sp")
        return (snapshot.registers.gpr[1] ?? 0) >>> 0;
    if (normalized === "pc")
        return snapshot.registers.pc >>> 0;
    if (normalized === "lr")
        return snapshot.registers.lr >>> 0;
    if (normalized === "ctr")
        return snapshot.registers.ctr >>> 0;
    if (normalized === "xer")
        return snapshot.registers.xer >>> 0;
    if (normalized === "cr")
        return snapshot.registers.cr >>> 0;
    if (normalized === "msr")
        return snapshot.registers.msr >>> 0;
    if (normalized.startsWith("0x")) {
        const parsed = Number.parseInt(normalized.slice(2), 16);
        return Number.isFinite(parsed) ? parsed >>> 0 : null;
    }
    if (/^\d+$/.test(normalized)) {
        const parsed = Number.parseInt(normalized, 10);
        return Number.isFinite(parsed) ? parsed >>> 0 : null;
    }
    return null;
};
const evaluateBreakpointCondition = (input, snapshot) => {
    const trimmed = input.trim();
    if (!trimmed) {
        return { kind: "empty" };
    }
    const condition = parseBreakpointCondition(trimmed);
    if (!condition) {
        return {
            kind: "error",
            message: "Use register/value comparisons like r3 == 42 or sp >= 0x80001000.",
        };
    }
    const left = resolveBreakpointOperand(snapshot, condition.left);
    const right = resolveBreakpointOperand(snapshot, condition.right);
    if (left == null || right == null) {
        return {
            kind: "error",
            message: `Unknown operand in \"${trimmed}\". Supported names: r0-r31, sp, pc, lr, ctr, xer, cr, msr.`,
        };
    }
    const matches = (() => {
        switch (condition.operator) {
            case "==": return left === right;
            case "!=": return left !== right;
            case "<": return left < right;
            case "<=": return left <= right;
            case ">": return left > right;
            case ">=": return left >= right;
        }
    })();
    return {
        kind: "ok",
        matches,
        message: `${condition.left} ${condition.operator} ${condition.right} → ${matches ? "true" : "false"} (${hex32(left)} vs ${hex32(right)})`,
    };
};
const mergeWriteRanges = (writes) => {
    const merged = [];
    const ranges = writes
        .filter((write) => Number.isFinite(write.addr) && Number.isFinite(write.size) && write.size > 0)
        .map((write) => {
        const addr = write.addr >>> 0;
        const size = write.size >>> 0;
        return { addr, end: addr + size };
    })
        .sort((left, right) => left.addr - right.addr || left.end - right.end);
    for (const range of ranges) {
        const previous = merged[merged.length - 1];
        if (!previous || range.addr > previous.addr + previous.size) {
            merged.push({ addr: range.addr, size: range.end - range.addr });
            continue;
        }
        previous.size = Math.max(previous.addr + previous.size, range.end) - previous.addr;
    }
    return merged;
};
const parseErrorAddress = (message) => {
    if (!message) {
        return null;
    }
    const match = ERROR_ADDRESS_RE.exec(message);
    if (!match) {
        return null;
    }
    const parsed = Number.parseInt(match[1] ?? "", 16);
    return Number.isFinite(parsed) ? parsed >>> 0 : null;
};
const isRuntimeErrorHaltReason = (reason) => {
    if (typeof reason === "string") {
        return reason === "Trap";
    }
    return "MemoryError" in reason || "InvalidInstruction" in reason || "Signal" in reason;
};
const runtimeErrorTitle = (reason, fallback) => {
    if (reason == null) {
        return fallback ? "Runtime Error" : "No Error";
    }
    if (typeof reason === "string") {
        return reason === "Trap" ? "Program Trap" : "Runtime Error";
    }
    if ("MemoryError" in reason)
        return "Memory Access Error";
    if ("InvalidInstruction" in reason)
        return "Invalid Instruction";
    if ("Signal" in reason)
        return "Signal Stop";
    return fallback ? "Runtime Error" : "Execution Halt";
};
const findSourceMapEntryForPc = (entries, pc) => {
    const address = pc >>> 0;
    for (const entry of entries) {
        const start = entry.start_addr >>> 0;
        const end = (start + (entry.byte_len >>> 0)) >>> 0;
        if (address >= start && address < end) {
            return entry;
        }
    }
    return null;
};
export const App = () => {
    let memoryJumpToken = 0;
    let disasmJumpToken = 0;
    let performanceSampleId = 0;
    let runtimeErrorId = 0;
    let lastAutofocusedRuntimeErrorId = 0;
    let performanceAnchor = null;
    let emulatorPerformanceStart = null;
    let emulatorPendingDeltaSteps = null;
    const history = new SnapshotHistory(200);
    const [state, setState] = createStore(structuredClone(EMPTY_STATE));
    const [source, setSource] = createSignal(DEFAULT_SOURCE);
    const [assembledSourceText, setAssembledSourceText] = createSignal(DEFAULT_SOURCE);
    const [assembleErrors, setAssembleErrors] = createSignal([]);
    const [assembled, setAssembled] = createSignal(null);
    const [disasm, setDisasm] = createSignal([]);
    const [executableRanges, setExecutableRanges] = createSignal([]);
    const [disassemblyLineLimit, setDisassemblyLineLimit] = createSignal(DEFAULT_DESKTOP_SETTINGS.disassembly_line_limit);
    const [errorContextSteps, setErrorContextSteps] = createSignal(DEFAULT_DESKTOP_SETTINGS.error_context_steps);
    const [trace, setTrace] = createSignal([]);
    const [breakpointConditions, setBreakpointConditions] = createSignal({});
    const [performanceSamples, setPerformanceSamples] = createSignal([]);
    const [runtimeError, setRuntimeError] = createSignal(null);
    const [running, setRunning] = createSignal(false);
    const [emulatorLaunch, setEmulatorLaunch] = createSignal(null);
    const [emulatorRunning, setEmulatorRunning] = createSignal(false);
    const [lastEmulatorStopReason, setLastEmulatorStopReason] = createSignal(null);
    const [lastRunMode, setLastRunMode] = createSignal("run");
    const [historyIndex, setHistoryIndex] = createSignal(-1);
    const [historyTotal, setHistoryTotal] = createSignal(0);
    const [disasmJumpRequest, setDisasmJumpRequest] = createSignal(null);
    const [memoryJumpRequest, setMemoryJumpRequest] = createSignal(null);
    const [progressState, setProgressState] = createSignal(null);
    const [statusMsg, setStatusMsg] = createSignal(null);
    const sessionActive = () => emulatorLaunch() !== null;
    const controlsLocked = () => running() || sessionActive();
    const executionRunning = () => running() || emulatorRunning();
    const historyAtLive = () => historyTotal() > 0 && historyIndex() === historyTotal() - 1;
    const viewingHistory = () => sessionActive() && historyTotal() > 0 && !historyAtLive();
    const hasLoadedExecutableProgram = () => executableRanges().length > 0 || disasm().length > 0;
    const replaceState = (s) => setState(produce((d) => Object.assign(d, s)));
    const currentHistoryEntry = () => history.current;
    const syncHistoryMeta = () => {
        setHistoryIndex(history.currentIndex);
        setHistoryTotal(history.length);
    };
    const clearHistory = () => {
        history.clear();
        syncHistoryMeta();
    };
    const resetPerformanceTimeline = () => {
        performanceSampleId = 0;
        performanceAnchor = null;
        emulatorPerformanceStart = null;
        emulatorPendingDeltaSteps = null;
        setPerformanceSamples([]);
    };
    const recordPerformanceSample = (snapshot, source, label, options) => {
        const timestampMs = nowMs();
        const previous = performanceAnchor && performanceAnchor.source === source ? performanceAnchor : null;
        const elapsedMs = options?.startedAt != null
            ? Math.max(0, timestampMs - options.startedAt)
            : previous
                ? Math.max(0, timestampMs - previous.timestampMs)
                : 0;
        const deltaSteps = options && "deltaSteps" in options
            ? options.deltaSteps ?? null
            : previous
                ? Math.max(0, snapshot.step_count - previous.stepCount)
                : 0;
        const instructionsPerSecond = deltaSteps != null && elapsedMs > 0
            ? deltaSteps / (elapsedMs / 1000)
            : null;
        const sample = {
            id: ++performanceSampleId,
            source,
            label,
            timestampMs,
            elapsedMs,
            stepCount: snapshot.step_count,
            deltaSteps,
            instructionsPerSecond,
            pc: snapshot.registers.pc >>> 0,
            note: options?.note,
        };
        performanceAnchor = { source, timestampMs, stepCount: snapshot.step_count };
        setPerformanceSamples((current) => [...current.slice(-119), sample]);
    };
    const breakpointConditionFor = (addr) => breakpointConditions()[addr >>> 0] ?? "";
    const disasmLineByAddr = createMemo(() => {
        const map = new Map();
        for (const line of disasm()) {
            map.set(line.address >>> 0, line);
        }
        return map;
    });
    const programBaseAddr = createMemo(() => executableRanges()[0]?.base ?? disasm()[0]?.address ?? BASE_ADDR);
    const pausedAtBreakpoint = createMemo(() => {
        const reason = lastEmulatorStopReason();
        return sessionActive() && !emulatorRunning() && !!reason && typeof reason !== "string" && "Breakpoint" in reason;
    });
    const assembledSourceLines = createMemo(() => assembledSourceText().split("\n"));
    const clearRuntimeError = () => setRuntimeError(null);
    const recordRuntimeError = (source, snapshot, traceEntries, reason, fallback) => {
        const summary = reason ? formatHaltReasonLabel(reason) : (fallback ?? "Runtime Error");
        const sourceMapEntry = assembled()?.source_map
            ? findSourceMapEntryForPc(assembled().source_map, snapshot.registers.pc >>> 0)
            : null;
        const faultingTrace = [...traceEntries]
            .reverse()
            .find((entry) => (entry.pc >>> 0) === (snapshot.registers.pc >>> 0))
            ?? traceEntries.at(-1)
            ?? null;
        const line = disasmLineByAddr().get(snapshot.registers.pc >>> 0);
        const previousSteps = faultingTrace
            ? traceEntries.filter((entry) => entry.step < faultingTrace.step).slice(-errorContextSteps())
            : traceEntries.slice(-errorContextSteps());
        runtimeErrorId += 1;
        setRuntimeError({
            id: runtimeErrorId,
            title: runtimeErrorTitle(reason, fallback),
            summary,
            source,
            pc: snapshot.registers.pc >>> 0,
            stepCount: snapshot.step_count,
            assembledSourceLocation: sourceMapEntry
                ? {
                    line: sourceMapEntry.line,
                    text: assembledSourceLines()[Math.max(0, sourceMapEntry.line - 1)]?.trimEnd() ?? "",
                    startAddr: sourceMapEntry.start_addr >>> 0,
                    endAddr: ((sourceMapEntry.start_addr >>> 0) + (sourceMapEntry.byte_len >>> 0)) >>> 0,
                }
                : null,
            affectedAddress: reason && typeof reason !== "string" && "MemoryError" in reason
                ? parseErrorAddress(reason.MemoryError)
                : parseErrorAddress(fallback),
            faultingInstruction: faultingTrace
                ? {
                    step: faultingTrace.step,
                    raw: faultingTrace.raw >>> 0,
                    mnemonic: faultingTrace.mnemonic,
                    operands: faultingTrace.operands,
                }
                : line
                    ? {
                        raw: line.raw >>> 0,
                        mnemonic: line.mnemonic,
                        operands: line.operands,
                    }
                    : null,
            previousSteps,
        });
    };
    const setExecutableRangesFromDisasm = (ranges) => {
        const normalized = [...ranges]
            .filter((range) => range.length > 0)
            .map((range) => ({ ...range, base: range.base >>> 0, length: range.length >>> 0 }))
            .sort((left, right) => left.base - right.base);
        setExecutableRanges(normalized);
    };
    const requestDisassemblyJump = (addr) => {
        disasmJumpToken += 1;
        setDisasmJumpRequest({ addr: addr >>> 0, token: disasmJumpToken });
    };
    const setBreakpointCondition = (addr, value) => {
        setBreakpointConditions((current) => {
            const key = addr >>> 0;
            const next = { ...current };
            if (value.trim()) {
                next[key] = value;
            }
            else {
                delete next[key];
            }
            return next;
        });
    };
    const clearBreakpointCondition = (addr) => {
        setBreakpointConditions((current) => {
            const key = addr >>> 0;
            if (!(key in current)) {
                return current;
            }
            const next = { ...current };
            delete next[key];
            return next;
        });
    };
    const showSnapshot = (entry) => {
        const snapshot = "snapshot" in entry ? entry.snapshot : entry;
        replaceState(structuredClone(snapshot));
        if ("disasmLines" in entry) {
            setDisasm(entry.disasmLines);
        }
    };
    const showLiveSnapshot = () => {
        const entry = history.toLive();
        if (!entry) {
            return;
        }
        syncHistoryMeta();
        showSnapshot(entry);
        setStatusMsg(`Returned to live emulator state at ${hex32(entry.snapshot.registers.pc)}.`);
    };
    const markEmulatorRunning = () => {
        setEmulatorRunning(true);
        setLastEmulatorStopReason(null);
        setState("halted", false);
        setState("halt_reason", "Running");
    };
    const describeEmulatorStop = (result) => {
        if (result.reason === "InitialStop") {
            return `Connected to Dolphin at ${hex32(result.snapshot.registers.pc)}.`;
        }
        if (result.reason === "Step") {
            return `Stepped Dolphin to ${hex32(result.snapshot.registers.pc)}.`;
        }
        if (result.reason === "ManualBreak") {
            return `Paused Dolphin at ${hex32(result.snapshot.registers.pc)}.`;
        }
        if ("Breakpoint" in result.reason) {
            return `Dolphin hit a breakpoint at ${hex32(result.reason.Breakpoint)}.`;
        }
        if ("Watchpoint" in result.reason) {
            const stop = result.reason.Watchpoint;
            const suffix = stop.exception_code ? ` (exception ${stop.exception_code})` : "";
            return `Dolphin hit a ${stop.kind.toLowerCase()} watchpoint at ${hex32(stop.address)}${suffix}.`;
        }
        if ("Signal" in result.reason) {
            const stop = result.reason.Signal;
            const suffix = stop.exception_code ? ` (exception ${stop.exception_code})` : "";
            return `Dolphin stopped with signal ${stop.signal}${suffix} at ${hex32(result.snapshot.registers.pc)}.`;
        }
        return `Dolphin stopped at ${hex32(result.snapshot.registers.pc)}.`;
    };
    const captureMemoryRange = async (base, length) => {
        if (length <= 0) {
            return new Uint8Array();
        }
        try {
            return new Uint8Array(await api.emulatorReadMemory(base >>> 0, length >>> 0));
        }
        catch {
            return new Uint8Array();
        }
    };
    const readCachedByte = (ranges, addr) => {
        const address = addr >>> 0;
        for (const range of ranges) {
            const offset = address - range.base;
            if (offset >= 0 && offset < range.bytes.length) {
                return range.bytes[offset];
            }
        }
        return undefined;
    };
    const readCachedRanges = (ranges, addr, len) => {
        const start = addr >>> 0;
        const length = len >>> 0;
        const out = new Uint8Array(length);
        for (const range of ranges) {
            const rangeEnd = (range.base + range.bytes.length) >>> 0;
            if (!range.bytes.length || start >= rangeEnd || start + length <= range.base) {
                continue;
            }
            const copyStart = Math.max(start, range.base);
            const copyEnd = Math.min(start + length, rangeEnd);
            const sourceOffset = copyStart - range.base;
            const targetOffset = copyStart - start;
            out.set(range.bytes.slice(sourceOffset, sourceOffset + (copyEnd - copyStart)), targetOffset);
        }
        return out;
    };
    const deriveHistoryWrites = (currentRanges, previousEntry) => {
        if (!previousEntry) {
            return [];
        }
        const previousRanges = previousEntry.memoryRanges.filter((range) => range.kind !== "write");
        const writes = [];
        for (const range of currentRanges) {
            let runStart = -1;
            let runSize = 0;
            for (let offset = 0; offset < range.bytes.length; offset += 1) {
                const address = (range.base + offset) >>> 0;
                const previousByte = readCachedByte(previousRanges, address);
                const changed = previousByte !== undefined && previousByte !== range.bytes[offset];
                if (!changed) {
                    if (runStart >= 0) {
                        writes.push({ addr: runStart >>> 0, size: runSize >>> 0 });
                        runStart = -1;
                        runSize = 0;
                    }
                    continue;
                }
                if (runStart < 0) {
                    runStart = address;
                    runSize = 1;
                    continue;
                }
                runSize += 1;
            }
            if (runStart >= 0) {
                writes.push({ addr: runStart >>> 0, size: runSize >>> 0 });
            }
        }
        return mergeWriteRanges(writes);
    };
    const captureWriteHistoryRanges = async (writes, baseRanges) => {
        const ranges = [];
        for (const [index, write] of mergeWriteRanges(writes).entries()) {
            const writeEnd = write.addr + write.size;
            const covered = baseRanges.some((range) => write.addr < range.base + range.bytes.length && writeEnd > range.base);
            const bytes = covered
                ? readCachedRanges(baseRanges, write.addr, write.size)
                : await captureMemoryRange(write.addr, write.size);
            if (!bytes.length) {
                continue;
            }
            ranges.push({
                kind: "write",
                label: `recent write #${index + 1}`,
                base: write.addr,
                bytes,
            });
        }
        return ranges;
    };
    const captureHistoryMemory = async (snapshot) => {
        const ranges = [];
        const programRanges = executableRanges().length
            ? executableRanges()
            : [{ label: "program", base: BASE_ADDR, length: Math.max(0, (snapshot.program_end >>> 0) - BASE_ADDR) }];
        for (const range of programRanges) {
            const programBytes = await captureMemoryRange(range.base, range.length);
            if (!programBytes.length) {
                continue;
            }
            ranges.push({
                kind: "program",
                label: range.label,
                base: range.base,
                bytes: programBytes,
            });
        }
        const stackPointer = (snapshot.registers.gpr[1] ?? 0) >>> 0;
        if (stackPointer >= BASE_ADDR && stackPointer < RAM_END) {
            const halfWindow = STACK_WINDOW_BYTES >>> 1;
            const stackBase = Math.max(BASE_ADDR, (stackPointer - halfWindow) >>> 0);
            const stackEnd = Math.min(RAM_END, stackBase + STACK_WINDOW_BYTES);
            const stackBytes = await captureMemoryRange(stackBase, stackEnd - stackBase);
            if (stackBytes.length) {
                ranges.push({
                    kind: "stack",
                    label: "stack",
                    base: stackBase >>> 0,
                    bytes: stackBytes,
                });
            }
        }
        return ranges;
    };
    const captureHistoryDisassembly = async (memoryRanges) => {
        const programRanges = memoryRanges.filter((range) => range.kind === "program" && range.bytes.length);
        if (!programRanges.length) {
            return [];
        }
        try {
            const chunks = await Promise.all(programRanges.map((range) => api.disassemble(Array.from(range.bytes), range.base)));
            return chunks.flat().sort((left, right) => left.address - right.address);
        }
        catch {
            return [];
        }
    };
    const buildHistoryEntry = async (snapshot) => {
        const baseRanges = await captureHistoryMemory(snapshot);
        const previousEntry = history.current ?? undefined;
        const lastWrites = snapshot.last_writes.length
            ? mergeWriteRanges(snapshot.last_writes)
            : deriveHistoryWrites(baseRanges, previousEntry);
        const memoryRanges = baseRanges.concat(await captureWriteHistoryRanges(lastWrites, baseRanges));
        return {
            snapshot: {
                ...structuredClone(snapshot),
                last_writes: lastWrites,
            },
            memoryRanges,
            disasmLines: await captureHistoryDisassembly(baseRanges),
        };
    };
    const continuePastConditionalBreakpoint = async (addr, conditionText) => {
        try {
            await api.emulatorClearBreakpoint(addr >>> 0);
            const stepResult = await api.emulatorStep();
            setState("breakpoints", await api.emulatorSetBreakpoint(addr >>> 0));
            if (stepResult.reason !== "Step") {
                await applyEmulatorStop(stepResult);
                return true;
            }
            await api.emulatorContinue();
            markEmulatorRunning();
            setStatusMsg(`Breakpoint ${hex32(addr)} skipped because ${conditionText} evaluated false.`);
            return true;
        }
        catch (err) {
            try {
                setState("breakpoints", await api.emulatorSetBreakpoint(addr >>> 0));
            }
            catch {
                // Ignore restore failures after a conditional-breakpoint error.
            }
            setStatusMsg(`conditional breakpoint: ${String(err)}`);
            return false;
        }
    };
    const maybeContinueConditionalBreakpoint = async (result) => {
        if (typeof result.reason === "string" || !("Breakpoint" in result.reason)) {
            return false;
        }
        const addr = result.reason.Breakpoint >>> 0;
        const conditionText = breakpointConditionFor(addr).trim();
        if (!conditionText) {
            return false;
        }
        const evaluation = evaluateBreakpointCondition(conditionText, result.snapshot);
        if (evaluation.kind === "error") {
            setStatusMsg(`Conditional breakpoint ${hex32(addr)}: ${evaluation.message}`);
            return false;
        }
        if (evaluation.kind !== "ok" || evaluation.matches) {
            return false;
        }
        return continuePastConditionalBreakpoint(addr, conditionText);
    };
    const applyEmulatorStop = async (result) => {
        if (await maybeContinueConditionalBreakpoint(result)) {
            return;
        }
        const entry = await buildHistoryEntry(result.snapshot);
        history.push(entry);
        syncHistoryMeta();
        showSnapshot(entry);
        setEmulatorRunning(false);
        setLastEmulatorStopReason(result.reason);
        recordPerformanceSample(entry.snapshot, "emulator", (() => {
            if (result.reason === "InitialStop")
                return "Attach";
            if (result.reason === "Step")
                return "Step";
            if (result.reason === "ManualBreak")
                return "Pause";
            if ("Breakpoint" in result.reason)
                return "Breakpoint";
            if ("Watchpoint" in result.reason)
                return `${result.reason.Watchpoint.kind} watch`;
            return "Signal";
        })(), {
            startedAt: emulatorPerformanceStart,
            deltaSteps: emulatorPendingDeltaSteps,
            note: describeEmulatorStop(result),
        });
        emulatorPerformanceStart = null;
        emulatorPendingDeltaSteps = null;
        setStatusMsg(describeEmulatorStop(result));
    };
    const readHistoryMemory = (entry, addr, len) => {
        return readCachedRanges(entry.memoryRanges, addr, len);
    };
    const requestMemoryJump = (addr, label) => {
        memoryJumpToken += 1;
        setMemoryJumpRequest({ addr: addr >>> 0, token: memoryJumpToken });
        setStatusMsg(`Memory jump: ${label} at ${hex32(addr)}.`);
    };
    const onHistoryBack = () => {
        if (emulatorRunning())
            return;
        const entry = history.back();
        if (!entry)
            return;
        syncHistoryMeta();
        showSnapshot(entry);
        setStatusMsg(`Viewing snapshot ${history.currentIndex + 1} / ${history.length}. Memory panel shows cached program and stack bytes.`);
    };
    const onHistoryForward = () => {
        if (emulatorRunning())
            return;
        const entry = history.forward();
        if (!entry)
            return;
        syncHistoryMeta();
        showSnapshot(entry);
        setStatusMsg(history.isAtLive
            ? `Returned to live emulator state at ${hex32(entry.snapshot.registers.pc)}.`
            : `Viewing snapshot ${history.currentIndex + 1} / ${history.length}. Memory panel shows cached program and stack bytes.`);
    };
    const refreshTrace = async () => {
        try {
            const nextTrace = await api.getTrace(200);
            setTrace(nextTrace);
            return nextTrace;
        }
        catch (err) {
            console.warn("getTrace failed", err);
            return trace();
        }
    };
    const onAssemble = async () => {
        if (controlsLocked())
            return;
        setStatusMsg(null);
        try {
            const result = await api.assemble(source());
            setAssembled(result);
            setAssembledSourceText(source());
            setAssembleErrors(result.errors);
            setStatusMsg(result.ok
                ? `Assembled ${result.bytes.length} bytes, ${result.symbols.length} symbols.`
                : `Assemble failed: ${result.errors.length} error(s).`);
        }
        catch (err) {
            setStatusMsg(`assemble: ${String(err)}`);
        }
    };
    const onLoad = async () => {
        if (controlsLocked())
            return;
        const a = assembled();
        if (!a || !a.ok)
            await onAssemble();
        const ready = assembled();
        if (!ready || !ready.ok) {
            setStatusMsg("Cannot load: assembly failed.");
            return;
        }
        try {
            const snap = await api.loadProgram(ready.bytes, ready.symbols);
            resetPerformanceTimeline();
            clearRuntimeError();
            replaceState(snap);
            setLastEmulatorStopReason(null);
            setDisasm(await api.disassemble(ready.bytes, ready.base_addr));
            setExecutableRangesFromDisasm([{ label: "program", base: ready.base_addr, length: ready.bytes.length }]);
            setTrace([]);
            requestDisassemblyJump(ready.base_addr);
            setStatusMsg(`Loaded ${ready.bytes.length} bytes at ${hex32(ready.base_addr)}.`);
        }
        catch (err) {
            setStatusMsg(`load_program: ${String(err)}`);
        }
    };
    const onLoadBinary = async (file) => {
        if (controlsLocked())
            return;
        setStatusMsg(null);
        setProgressState({ value: 0.05, label: `Reading ${file.name}` });
        try {
            const bytes = Array.from(new Uint8Array(await file.arrayBuffer()));
            setProgressState({ value: 0.12, label: `Parsing ${file.name}` });
            const result = await api.loadBinary(bytes);
            const executableSections = result.sections.filter((section) => section.is_executable).length;
            resetPerformanceTimeline();
            clearRuntimeError();
            replaceState(result.snapshot);
            setLastEmulatorStopReason(null);
            setDisasm(result.disasm_lines);
            setExecutableRangesFromDisasm(result.sections
                .filter((section) => section.is_executable)
                .map((section) => ({
                label: section.name,
                base: section.load_addr,
                length: section.size,
            })));
            setTrace([]);
            setAssembled(null);
            setAssembledSourceText("");
            setAssembleErrors([]);
            requestDisassemblyJump(result.snapshot.registers.pc);
            setStatusMsg(`Loaded ${result.format} ${file.name}: ${result.sections.length} section(s), ${executableSections} executable. Entry ${hex32(result.snapshot.registers.pc)}.`);
        }
        catch (err) {
            setStatusMsg(`load_binary: ${String(err)}`);
        }
        finally {
            setProgressState(null);
        }
    };
    const onStep = async () => {
        if (running() || emulatorRunning())
            return;
        if (sessionActive()) {
            if (viewingHistory()) {
                showLiveSnapshot();
            }
            try {
                emulatorPerformanceStart = nowMs();
                emulatorPendingDeltaSteps = 1;
                await applyEmulatorStop(await api.emulatorStep());
            }
            catch (err) {
                emulatorPerformanceStart = null;
                emulatorPendingDeltaSteps = null;
                await handleEmulatorSessionFailure(String(err));
            }
            return;
        }
        try {
            const startedAt = nowMs();
            const r = await api.step();
            replaceState(r.state);
            setLastEmulatorStopReason(null);
            if (r.trace)
                setTrace((t) => [...t.slice(-199), r.trace]);
            recordPerformanceSample(r.state, "engine", "Step", {
                startedAt,
                deltaSteps: r.trace ? 1 : 0,
                note: r.error ?? undefined,
            });
            if (r.error) {
                const traceEntries = await refreshTrace();
                if (isRuntimeErrorHaltReason(r.state.halt_reason)) {
                    recordRuntimeError("Engine Step", r.state, traceEntries, r.state.halt_reason, r.error);
                }
                setStatusMsg(formatHaltReasonLabel(r.state.halt_reason));
            }
        }
        catch (err) {
            setStatusMsg(`step: ${String(err)}`);
        }
    };
    const onRun = async () => {
        if (controlsLocked())
            return;
        setRunning(true);
        setLastEmulatorStopReason(null);
        setStatusMsg(null);
        const startedAt = nowMs();
        try {
            const r = await api.runUntil(0);
            replaceState(r.state);
            const traceEntries = await refreshTrace();
            recordPerformanceSample(r.state, "engine", "Run", {
                startedAt,
                deltaSteps: r.steps_executed,
                note: formatHaltReasonLabel(r.halt_reason),
            });
            if (isRuntimeErrorHaltReason(r.halt_reason)) {
                recordRuntimeError("Engine Run", r.state, traceEntries, r.halt_reason);
            }
        }
        catch (err) {
            setStatusMsg(`run_until: ${String(err)}`);
        }
        finally {
            setRunning(false);
        }
    };
    const onRunWithEmulator = async () => {
        if (running())
            return;
        if (sessionActive()) {
            if (emulatorRunning())
                return;
            if (viewingHistory()) {
                showLiveSnapshot();
            }
            try {
                emulatorPerformanceStart = nowMs();
                emulatorPendingDeltaSteps = null;
                await api.emulatorContinue();
                markEmulatorRunning();
                setStatusMsg("Running in Dolphin...");
            }
            catch (err) {
                emulatorPerformanceStart = null;
                emulatorPendingDeltaSteps = null;
                await handleEmulatorSessionFailure(String(err));
            }
            return;
        }
        if (!hasLoadedExecutableProgram()) {
            await onLoad();
            if (!hasLoadedExecutableProgram()) {
                return;
            }
        }
        try {
            const launch = await api.launchWithEmulator();
            const initial = await api.connectGdb(launch.gdb_port);
            clearHistory();
            resetPerformanceTimeline();
            setEmulatorLaunch(launch);
            await applyEmulatorStop(initial);
            emulatorPerformanceStart = nowMs();
            emulatorPendingDeltaSteps = null;
            await api.emulatorContinue();
            markEmulatorRunning();
            setStatusMsg(`Running in Dolphin via ${launch.dolphin_path}.`);
        }
        catch (err) {
            try {
                await api.stopEmulator();
            }
            catch {
                // ignore cleanup errors when the launch/connect path already failed
            }
            setEmulatorLaunch(null);
            setEmulatorRunning(false);
            clearHistory();
            setStatusMsg(`launch_with_emulator: ${String(err)}`);
        }
    };
    const onNextBranch = async () => {
        if (!sessionActive() || emulatorRunning())
            return;
        if (viewingHistory()) {
            showLiveSnapshot();
        }
        const maxScanSteps = 50_000;
        const startedAt = nowMs();
        let stepsExecuted = 0;
        try {
            while (stepsExecuted < maxScanSteps) {
                const result = await api.emulatorStep();
                stepsExecuted += 1;
                if (result.reason !== "Step") {
                    emulatorPerformanceStart = startedAt;
                    emulatorPendingDeltaSteps = stepsExecuted;
                    await applyEmulatorStop(result);
                    return;
                }
                const nextLine = disasmLineByAddr().get(result.snapshot.registers.pc >>> 0);
                if (nextLine && nextLine.mnemonic.trim().toLowerCase().startsWith("b")) {
                    emulatorPerformanceStart = startedAt;
                    emulatorPendingDeltaSteps = stepsExecuted;
                    await applyEmulatorStop(result);
                    setStatusMsg(`Paused before branch at ${hex32(result.snapshot.registers.pc)}.`);
                    return;
                }
            }
            emulatorPerformanceStart = null;
            emulatorPendingDeltaSteps = null;
            setStatusMsg(`next_branch: exceeded ${maxScanSteps.toLocaleString()} instructions without reaching another branch.`);
        }
        catch (err) {
            emulatorPerformanceStart = null;
            emulatorPendingDeltaSteps = null;
            await handleEmulatorSessionFailure(String(err));
        }
    };
    const restoreBaseDisassembly = async () => {
        const ready = assembled();
        if (!ready?.ok) {
            return;
        }
        try {
            setDisasm(await api.disassemble(ready.bytes, ready.base_addr));
            setExecutableRangesFromDisasm([{ label: "program", base: ready.base_addr, length: ready.bytes.length }]);
        }
        catch {
            // Leave the last visible disassembly intact if reloading the base listing fails.
        }
    };
    const finalizeEmulatorSessionClose = async (message) => {
        clearRuntimeError();
        setEmulatorLaunch(null);
        setEmulatorRunning(false);
        setLastEmulatorStopReason(null);
        clearHistory();
        await restoreBaseDisassembly();
        setStatusMsg(message);
    };
    const handleEmulatorSessionFailure = async (message, options) => {
        if (options?.stopBackend !== false) {
            try {
                await api.stopEmulator();
            }
            catch {
                // Ignore cleanup failures when the emulator process has already gone away.
            }
        }
        await finalizeEmulatorSessionClose(`emulator: ${message}`);
    };
    const closeEmulatorSession = async () => {
        if (!sessionActive())
            return;
        let message = "Stopped Dolphin emulator session.";
        try {
            await api.stopEmulator();
        }
        catch (err) {
            message = `stop_emulator: ${String(err)}`;
        }
        finally {
            await finalizeEmulatorSessionClose(message);
        }
    };
    const onStopEmulator = async () => {
        if (!emulatorRunning())
            return;
        try {
            await api.emulatorBreak();
            setStatusMsg("Pause requested in Dolphin...");
        }
        catch (err) {
            setStatusMsg(`emulator_break: ${String(err)}`);
        }
    };
    const onReset = async () => {
        if (controlsLocked())
            return;
        try {
            resetPerformanceTimeline();
            clearRuntimeError();
            replaceState(await api.reset());
            setLastEmulatorStopReason(null);
            setTrace([]);
            setStatusMsg("Engine reset.");
        }
        catch (err) {
            setStatusMsg(`reset: ${String(err)}`);
        }
    };
    const onToggleBreakpoint = async (addr) => {
        if (executionRunning()) {
            setStatusMsg("Pause execution before editing breakpoints.");
            return;
        }
        if (viewingHistory()) {
            showLiveSnapshot();
        }
        const set = new Set(state.breakpoints.map((b) => b >>> 0));
        const existed = set.has(addr >>> 0);
        try {
            const next = sessionActive()
                ? (existed
                    ? await api.emulatorClearBreakpoint(addr >>> 0)
                    : await api.emulatorSetBreakpoint(addr >>> 0))
                : (existed
                    ? await api.clearBreakpoint(addr >>> 0)
                    : await api.setBreakpoint(addr >>> 0));
            setState("breakpoints", next);
            if (existed) {
                clearBreakpointCondition(addr);
            }
        }
        catch (err) {
            setStatusMsg(`breakpoint: ${String(err)}`);
        }
    };
    const readMemory = async (addr, len) => {
        if (sessionActive()) {
            if (viewingHistory()) {
                const entry = currentHistoryEntry();
                return entry ? readHistoryMemory(entry, addr >>> 0, len >>> 0) : new Uint8Array();
            }
            if (emulatorRunning())
                return new Uint8Array();
            try {
                return new Uint8Array(await api.emulatorReadMemory(addr >>> 0, len >>> 0));
            }
            catch {
                return new Uint8Array();
            }
        }
        try {
            return new Uint8Array(await api.readMemory(addr >>> 0, len >>> 0));
        }
        catch {
            return new Uint8Array();
        }
    };
    const onJump = (addr) => {
        requestDisassemblyJump(addr);
    };
    const onKey = (e) => {
        if (e.altKey && !e.ctrlKey && !e.metaKey && !e.shiftKey) {
            const key = e.key.toLowerCase();
            if (key === "p") {
                e.preventDefault();
                requestMemoryJump(programBaseAddr(), "Program");
                return;
            }
            if (key === "c") {
                e.preventDefault();
                requestMemoryJump(pc(), "PC");
                return;
            }
            if (key === "s" && stackPointer() !== 0) {
                e.preventDefault();
                requestMemoryJump(stackPointer(), "Stack");
                return;
            }
            if (key === "w" && recentWriteAddr() !== 0) {
                e.preventDefault();
                requestMemoryJump(recentWriteAddr(), "Recent Write");
                return;
            }
        }
        if (e.key === "Escape" && sessionActive()) {
            e.preventDefault();
            void closeEmulatorSession();
        }
        if (e.key === "F5") {
            e.preventDefault();
            if (running()) {
                return;
            }
            if (lastRunMode() === "run-with-emulator") {
                void onRunWithEmulator();
            }
            else {
                void onRun();
            }
        }
        else if (e.key === "F10") {
            e.preventDefault();
            void onStep();
        }
        else if (e.key === "F9") {
            e.preventDefault();
            if (!executionRunning()) {
                void onToggleBreakpoint(state.registers.pc >>> 0);
            }
        }
        else if (e.ctrlKey && (e.key === "b" || e.key === "B")) {
            e.preventDefault();
            void onAssemble();
        }
        else if (e.ctrlKey && (e.key === "r" || e.key === "R")) {
            e.preventDefault();
            void onReset();
        }
    };
    onMount(() => {
        let unlistenStopped;
        let unlistenError;
        let unlistenBinaryLoadProgress;
        let unlistenSettings;
        window.addEventListener("keydown", onKey);
        void api.getState().then(replaceState).catch(() => undefined);
        void loadDesktopSettings()
            .then((settings) => {
            setDisassemblyLineLimit(settings.disassembly_line_limit);
            setErrorContextSteps(settings.error_context_steps);
        })
            .catch(() => undefined);
        void listenForDesktopSettingsUpdates((settings) => {
            setDisassemblyLineLimit(settings.disassembly_line_limit);
            setErrorContextSteps(settings.error_context_steps);
        }).then((dispose) => {
            unlistenSettings = dispose;
        });
        void listen("emulator-stopped", (event) => {
            void applyEmulatorStop(event.payload);
        }).then((dispose) => {
            unlistenStopped = dispose;
        });
        void listen("emulator-error", (event) => {
            void handleEmulatorSessionFailure(event.payload, { stopBackend: false });
        }).then((dispose) => {
            unlistenError = dispose;
        });
        void listen("binary-load-progress", (event) => {
            setProgressState({
                value: Math.max(0, Math.min(1, event.payload.value)),
                label: event.payload.label,
            });
        }).then((dispose) => {
            unlistenBinaryLoadProgress = dispose;
        });
        onCleanup(() => {
            unlistenStopped?.();
            unlistenError?.();
            unlistenBinaryLoadProgress?.();
            unlistenSettings?.();
        });
    });
    onCleanup(() => window.removeEventListener("keydown", onKey));
    createEffect(() => {
        if (!sessionActive() || emulatorRunning()) {
            return;
        }
        let disposed = false;
        const interval = window.setInterval(() => {
            if (disposed) {
                return;
            }
            void api.probeEmulator()
                .then((message) => {
                if (message) {
                    void handleEmulatorSessionFailure(message, { stopBackend: false });
                }
            })
                .catch(() => undefined);
        }, 1000);
        onCleanup(() => {
            disposed = true;
            window.clearInterval(interval);
        });
    });
    createEffect(() => {
        const activeBreakpoints = new Set(state.breakpoints.map((addr) => addr >>> 0));
        setBreakpointConditions((current) => {
            let changed = false;
            const next = {};
            for (const [key, value] of Object.entries(current)) {
                const numericKey = Number(key) >>> 0;
                if (activeBreakpoints.has(numericKey)) {
                    next[numericKey] = value;
                }
                else {
                    changed = true;
                }
            }
            return changed ? next : current;
        });
    });
    createEffect(() => {
        const report = runtimeError();
        if (!report || report.id === lastAutofocusedRuntimeErrorId) {
            return;
        }
        lastAutofocusedRuntimeErrorId = report.id;
        requestDisassemblyJump(report.pc);
    });
    const pc = createMemo(() => state.registers.pc >>> 0);
    const stackPointer = createMemo(() => (state.registers.gpr[1] ?? 0) >>> 0);
    const recentWriteAddr = createMemo(() => {
        const writes = state.last_writes;
        return writes.length ? (writes[writes.length - 1]?.addr ?? 0) >>> 0 : 0;
    });
    const halted = createMemo(() => state.halted);
    const haltReason = createMemo(() => state.halt_reason);
    const displayHalted = createMemo(() => !executionRunning() && halted());
    const breakpointConditionStatuses = createMemo(() => {
        const statuses = {};
        for (const addr of state.breakpoints) {
            const condition = breakpointConditionFor(addr);
            if (!condition.trim()) {
                continue;
            }
            const evaluation = evaluateBreakpointCondition(condition, state);
            if (evaluation.kind === "error") {
                statuses[addr >>> 0] = { message: evaluation.message ?? "Invalid condition.", tone: "error" };
            }
            else if (evaluation.kind === "ok") {
                statuses[addr >>> 0] = {
                    message: evaluation.message ?? "Condition ready.",
                    tone: evaluation.matches ? "success" : "muted",
                };
            }
        }
        return statuses;
    });
    const memoryPresets = createMemo(() => [
        {
            label: "Program",
            addr: programBaseAddr(),
            title: `Jump to the loaded program base address at ${hex32(programBaseAddr())} (Alt+P)`,
        },
        {
            label: "PC",
            addr: pc(),
            title: `Jump to the current program counter at ${hex32(pc())} (Alt+C)`,
        },
        {
            label: "Stack",
            addr: stackPointer(),
            title: stackPointer() === 0
                ? "Stack pointer is not available yet"
                : `Jump to the current stack pointer at ${hex32(stackPointer())} (Alt+S)`,
            disabled: stackPointer() === 0,
        },
        {
            label: "Recent Write",
            addr: recentWriteAddr(),
            title: recentWriteAddr() === 0
                ? "No recent memory writes are available"
                : `Jump to the most recent write at ${hex32(recentWriteAddr())} (Alt+W)`,
            disabled: recentWriteAddr() === 0,
        },
    ]);
    const memoryCoverageRegions = createMemo(() => {
        if (!viewingHistory()) {
            return undefined;
        }
        const entry = currentHistoryEntry();
        if (!entry) {
            return undefined;
        }
        return entry.memoryRanges.map((range) => ({
            label: `${range.label} cache`,
            base: range.base,
            end: (range.base + range.bytes.length) >>> 0,
        }));
    });
    const historyBanner = createMemo(() => {
        const entry = currentHistoryEntry();
        if (!viewingHistory() || !entry) {
            return null;
        }
        const labels = Array.from(new Set(entry.memoryRanges.map((range) => range.label))).join(" + ");
        return `History mode: viewing snapshot ${historyIndex() + 1} / ${historyTotal()} at ${hex32(entry.snapshot.registers.pc)}. Memory panel is showing cached ${labels || "snapshot"} bytes, not live Dolphin memory.`;
    });
    const previousHistoryEntry = createMemo(() => {
        const index = historyIndex();
        return index > 0 ? history.at(index - 1) : null;
    });
    const snapshotDiffEntries = createMemo(() => {
        if (!sessionActive()) {
            return [];
        }
        const current = currentHistoryEntry();
        const previous = previousHistoryEntry();
        if (!current || !previous) {
            return [];
        }
        return current.snapshot.last_writes.slice(0, 24).map((write, index) => {
            const size = Math.max(1, write.size >>> 0);
            const sampleLength = Math.min(size, 12);
            const labels = current.memoryRanges
                .filter((range) => write.addr < range.base + range.bytes.length && write.addr + size > range.base)
                .map((range) => range.label);
            return {
                id: `${historyIndex()}:${index}:${write.addr}:${write.size}`,
                addr: write.addr >>> 0,
                size,
                label: labels.length ? Array.from(new Set(labels)).join(" + ") : undefined,
                beforeBytes: Array.from(readHistoryMemory(previous, write.addr, sampleLength)),
                afterBytes: Array.from(readHistoryMemory(current, write.addr, sampleLength)),
            };
        });
    });
    const snapshotDiffComparisonLabel = createMemo(() => {
        if (!sessionActive() || !previousHistoryEntry()) {
            return undefined;
        }
        return historyAtLive()
            ? `Live vs Snapshot ${historyIndex()}`
            : `Snapshot ${historyIndex() + 1} vs Snapshot ${historyIndex()}`;
    });
    // Reactive subtitle for the topbar (and document.title for taskbar)
    const subtitle = createMemo(() => `PC ${hex32(pc())}  ·  ${Number(state.step_count).toLocaleString()} steps`);
    createEffect(() => { document.title = `PPC-Bench — ${subtitle()}`; });
    return (<PPCBenchShell titleBar={<DesktopTopbar subtitle={subtitle()} onSettings={() => void openSettingsWindow()}/>} topBar={<>
          <ControlBar pc={pc()} stepCount={Number(state.step_count)} halted={displayHalted()} haltReason={haltReason()} running={executionRunning()} controlsLocked={controlsLocked()} runDisabled={executionRunning() || (!sessionActive() && halted())} stepDisabled={executionRunning() || (!sessionActive() && halted())} lastRunMode={lastRunMode()} historyIndex={historyIndex()} historyTotal={historyTotal()} historyAtLive={historyAtLive()} onAssemble={onAssemble} onLoad={onLoad} onLoadBinary={onLoadBinary} onStep={onStep} onNextBranch={sessionActive() ? onNextBranch : undefined} onRun={onRun} onResume={sessionActive() ? onRunWithEmulator : undefined} onRunWithEmulator={onRunWithEmulator} onSetRunMode={setLastRunMode} onHistoryBack={sessionActive() ? onHistoryBack : undefined} onHistoryForward={sessionActive() ? onHistoryForward : undefined} onHistoryLive={sessionActive() && !historyAtLive() ? () => showLiveSnapshot() : undefined} onStop={emulatorRunning() ? onStopEmulator : undefined} onCloseEmulator={sessionActive() ? closeEmulatorSession : undefined} showPausedExecutionActions={pausedAtBreakpoint()} onReset={onReset} onManuals={() => void openManualsWindow()}/>
          {historyBanner() && (<div class="history-banner" role="status" aria-live="polite">
              {historyBanner()}
            </div>)}
          {progressState() && (<div class="task-progress" role="progressbar" aria-valuemin={0} aria-valuemax={100} aria-valuenow={Math.round(progressState().value * 100)}>
              <div class="task-progress__meta">
                <span>{progressState().label}</span>
                <span>{Math.round(progressState().value * 100)}%</span>
              </div>
              <div class="task-progress__track">
                <div class="task-progress__bar" style={{ width: `${Math.round(progressState().value * 100)}%` }}/>
              </div>
            </div>)}
        </>} left={
        // Single panel fills the column — no ResizableColumn needed.
        <CodeEditorPanel source={source()} onSourceChange={setSource} errors={assembleErrors()} readOnly={controlsLocked()}/>} center={<ResizableColumn items={[
                {
                    node: (<DisassemblyPanel lines={disasm()} pc={pc()} breakpoints={state.breakpoints} symbols={state.symbols} onToggleBreakpoint={(a) => void onToggleBreakpoint(a)} onJump={onJump} jumpRequest={disasmJumpRequest()} lineLimit={disassemblyLineLimit()}/>),
                    initialHeight: 460,
                },
                {
                    node: (<MemoryPanel onReadMemory={readMemory} lastWrites={state.last_writes} initialAddr={programBaseAddr()} presets={memoryPresets()} coverageRegions={memoryCoverageRegions()} jumpRequest={memoryJumpRequest()} refreshKey={sessionActive() ? `${historyIndex()}:${historyTotal()}:${emulatorRunning() ? "run" : "pause"}` : undefined}/>),
                    initialHeight: 230,
                },
                {
                    node: (<SnapshotDiffPanel entries={snapshotDiffEntries()} comparisonLabel={snapshotDiffComparisonLabel()} onJump={(addr) => requestMemoryJump(addr, "Diff")}/>),
                    // No initialHeight → grows to fill remaining space
                },
            ]}/>} right={<ResizableColumn items={[
                {
                    node: <RegistersPanel registers={state.registers}/>,
                    initialHeight: 320,
                },
                {
                    node: <FPUPanel fpu={state.fpu}/>,
                    initialHeight: 240,
                },
                {
                    node: <CallStackPanel frames={state.call_stack} onJump={onJump}/>,
                    initialHeight: 140,
                },
                {
                    node: (<BreakpointsPanel breakpoints={state.breakpoints} symbols={state.symbols} onClear={(a) => void onToggleBreakpoint(a)} onJump={onJump} enableConditions={sessionActive()} conditionValues={breakpointConditions()} conditionStatuses={breakpointConditionStatuses()} onConditionChange={setBreakpointCondition}/>),
                    initialHeight: 180,
                },
                {
                    node: <SymbolTablePanel symbols={state.symbols} onJump={onJump}/>,
                    // No initialHeight → grows to fill remaining space
                },
            ]}/>} bottom={<>
          {statusMsg() && (<div style="padding:4px 12px;background:var(--color-warning-bg);border:1px solid var(--color-warning-border);border-radius:var(--radius-sm);color:var(--color-text-muted);font:500 var(--size-label) var(--font-ui);margin-bottom:6px;flex-shrink:0;white-space:pre-wrap;overflow-wrap:anywhere;">
              {statusMsg()}
            </div>)}
          <DiagnosticsPanel performanceSamples={performanceSamples()} runtimeError={runtimeError()} errorContextSteps={errorContextSteps()} onJump={onJump} onClearError={clearRuntimeError}/>
        </>} initialLeftWidth={460} initialRightWidth={360} initialBottomHeight={200}/>);
};
