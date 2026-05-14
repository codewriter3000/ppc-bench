import { createSignal, createMemo, createEffect, onCleanup, onMount } from "solid-js";
import { createStore, produce } from "solid-js/store";
import { BreakpointsPanel, CallStackPanel, CodeEditorPanel, ControlBar, DisassemblyPanel, ExecutionLogPanel, FPUPanel, MemoryPanel, PPCBenchShell, RegistersPanel, ResizableColumn, SymbolTablePanel, } from "@ppc-bench/ui";
import { DesktopTopbar } from "./DesktopTopbar";
import { api } from "./tauri";
const BASE_ADDR = 0x8000_0000 >>> 0;
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
export const App = () => {
    const [state, setState] = createStore(structuredClone(EMPTY_STATE));
    const [source, setSource] = createSignal(DEFAULT_SOURCE);
    const [assembleErrors, setAssembleErrors] = createSignal([]);
    const [assembled, setAssembled] = createSignal(null);
    const [disasm, setDisasm] = createSignal([]);
    const [trace, setTrace] = createSignal([]);
    const [running, setRunning] = createSignal(false);
    const [statusMsg, setStatusMsg] = createSignal(null);
    const replaceState = (s) => setState(produce((d) => Object.assign(d, s)));
    const refreshTrace = async () => {
        try {
            setTrace(await api.getTrace(200));
        }
        catch (err) {
            console.warn("getTrace failed", err);
        }
    };
    const onAssemble = async () => {
        setStatusMsg(null);
        try {
            const result = await api.assemble(source());
            setAssembled(result);
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
            replaceState(snap);
            setDisasm(await api.disassemble(ready.bytes, ready.base_addr));
            setTrace([]);
            setStatusMsg(`Loaded ${ready.bytes.length} bytes at ${hex32(ready.base_addr)}.`);
        }
        catch (err) {
            setStatusMsg(`load_program: ${String(err)}`);
        }
    };
    const onStep = async () => {
        if (running())
            return;
        try {
            const r = await api.step();
            replaceState(r.state);
            if (r.trace)
                setTrace((t) => [...t.slice(-199), r.trace]);
            if (r.error)
                setStatusMsg(r.error);
        }
        catch (err) {
            setStatusMsg(`step: ${String(err)}`);
        }
    };
    const onRun = async () => {
        if (running())
            return;
        setRunning(true);
        setStatusMsg(null);
        try {
            const r = await api.runUntil(0);
            replaceState(r.state);
            await refreshTrace();
        }
        catch (err) {
            setStatusMsg(`run_until: ${String(err)}`);
        }
        finally {
            setRunning(false);
        }
    };
    const onReset = async () => {
        try {
            replaceState(await api.reset());
            setTrace([]);
            setStatusMsg("Engine reset.");
        }
        catch (err) {
            setStatusMsg(`reset: ${String(err)}`);
        }
    };
    const onToggleBreakpoint = async (addr) => {
        const set = new Set(state.breakpoints.map((b) => b >>> 0));
        try {
            const next = set.has(addr >>> 0)
                ? await api.clearBreakpoint(addr >>> 0)
                : await api.setBreakpoint(addr >>> 0);
            setState("breakpoints", next);
        }
        catch (err) {
            setStatusMsg(`breakpoint: ${String(err)}`);
        }
    };
    const readMemory = async (addr, len) => {
        try {
            return new Uint8Array(await api.readMemory(addr >>> 0, len >>> 0));
        }
        catch {
            return new Uint8Array();
        }
    };
    const onJump = (addr) => {
        document
            .querySelector(`[data-disasm-addr="${addr >>> 0}"]`)
            ?.scrollIntoView({ block: "center" });
    };
    const onKey = (e) => {
        if (e.key === "F5") {
            e.preventDefault();
            void onRun();
        }
        else if (e.key === "F10") {
            e.preventDefault();
            void onStep();
        }
        else if (e.key === "F9") {
            e.preventDefault();
            void onToggleBreakpoint(state.registers.pc >>> 0);
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
        window.addEventListener("keydown", onKey);
        void api.getState().then(replaceState).catch(() => undefined);
    });
    onCleanup(() => window.removeEventListener("keydown", onKey));
    const pc = createMemo(() => state.registers.pc >>> 0);
    const halted = createMemo(() => state.halted);
    const haltReason = createMemo(() => state.halt_reason);
    // Reactive subtitle for the topbar (and document.title for taskbar)
    const subtitle = createMemo(() => `PC ${hex32(pc())}  ·  ${Number(state.step_count).toLocaleString()} steps`);
    createEffect(() => { document.title = `PPC-Bench — ${subtitle()}`; });
    return (<PPCBenchShell titleBar={<DesktopTopbar subtitle={subtitle()}/>} topBar={<ControlBar pc={pc()} stepCount={Number(state.step_count)} halted={halted()} haltReason={haltReason()} running={running()} onAssemble={onAssemble} onLoad={onLoad} onStep={onStep} onRun={onRun} onReset={onReset}/>} left={
        // Single panel fills the column — no ResizableColumn needed.
        <CodeEditorPanel source={source()} onSourceChange={setSource} errors={assembleErrors()} readOnly={running()}/>} center={<ResizableColumn items={[
                {
                    node: (<DisassemblyPanel lines={disasm()} pc={pc()} breakpoints={state.breakpoints} symbols={state.symbols} onToggleBreakpoint={(a) => void onToggleBreakpoint(a)}/>),
                    initialHeight: 500,
                },
                {
                    node: (<MemoryPanel onReadMemory={readMemory} lastWrites={state.last_writes} initialAddr={BASE_ADDR}/>),
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
                    node: (<BreakpointsPanel breakpoints={state.breakpoints} symbols={state.symbols} onClear={(a) => void onToggleBreakpoint(a)} onJump={onJump}/>),
                    initialHeight: 120,
                },
                {
                    node: <SymbolTablePanel symbols={state.symbols} onJump={onJump}/>,
                    // No initialHeight → grows to fill remaining space
                },
            ]}/>} bottom={<>
          {statusMsg() && (<div style="padding:4px 12px;background:var(--color-warning-bg);border:1px solid var(--color-warning-border);border-radius:var(--radius-sm);color:var(--color-text-muted);font:500 var(--size-label) var(--font-ui);margin-bottom:6px;flex-shrink:0;">
              {statusMsg()}
            </div>)}
          <ExecutionLogPanel trace={trace()} onJump={onJump}/>
        </>} initialLeftWidth={460} initialRightWidth={360} initialBottomHeight={200}/>);
};
