import { type Component, Show, createSignal } from "solid-js";
import type { HaltReason } from "@ppc-bench/kernel";
import { haltReasonLabel } from "@ppc-bench/kernel";
import { hex32 } from "../styles/format";
import "../styles/control-bar.css";

export type RunMode = "run" | "run-with-emulator";

export interface ControlBarProps {
  pc: number;
  stepCount: number;
  halted: boolean;
  haltReason: HaltReason;
  running: boolean;
  controlsLocked?: boolean;
  runDisabled?: boolean;
  stepDisabled?: boolean;
  lastRunMode?: RunMode;
  historyIndex?: number;
  historyTotal?: number;
  historyAtLive?: boolean;
  onAssemble: () => void;
  onLoad: () => void;
  onLoadBinary?: (file: File) => void | Promise<void>;
  onStep: () => void;
  onNextBranch?: () => void;
  onRun: () => void;
  onResume?: () => void;
  onRunWithEmulator?: () => void;
  onSetRunMode?: (mode: RunMode) => void;
  onHistoryBack?: () => void;
  onHistoryForward?: () => void;
  onHistoryLive?: () => void;
  onStop?: () => void;
  onCloseEmulator?: () => void;
  showPausedExecutionActions?: boolean;
  onReset: () => void;
  onManuals?: () => void;
}

export const ControlBar: Component<ControlBarProps> = (props) => {
  let binaryInputRef: HTMLInputElement | undefined;
  const [menuOpen, setMenuOpen] = createSignal(false);

  const currentRunMode = () => props.lastRunMode ?? "run";
  const runUsesEmulator = () => currentRunMode() === "run-with-emulator";
  const primaryRunLabel = () => (runUsesEmulator() ? "▶ Run + Emulator" : "▶ Run");
  const primaryRunTitle = () =>
    runUsesEmulator() ? "Run in Dolphin emulator (F5)" : "Run until halt (F5)";
  const controlsLocked = () => props.controlsLocked ?? props.running;
  const primaryRunDisabled = () =>
    props.runDisabled
    ?? (props.running || props.halted || (runUsesEmulator() && !props.onRunWithEmulator));
  const stepDisabled = () => props.stepDisabled ?? (props.running || props.halted);
  const showPausedExecutionActions = () => props.showPausedExecutionActions === true;
  const historyVisible = () => (props.historyTotal ?? 0) > 0;
  const historyIndexLabel = () => `${(props.historyIndex ?? 0) + 1} / ${props.historyTotal ?? 0}`;

  const closeMenu = () => setMenuOpen(false);

  const runSelectedMode = () => {
    if (runUsesEmulator()) {
      props.onRunWithEmulator?.();
      return;
    }
    props.onRun();
  };

  const selectRunMode = (mode: RunMode) => {
    props.onSetRunMode?.(mode);
    closeMenu();
    if (mode === "run-with-emulator") {
      props.onRunWithEmulator?.();
      return;
    }
    props.onRun();
  };

  const pillClass = () =>
    props.halted ? "pill pill--halted" : props.running ? "pill pill--running" : "pill pill--ready";

  return (
    <div class="control-bar" role="toolbar" aria-label="PPC-Bench controls">
      <input
        ref={binaryInputRef}
        type="file"
        accept=".dol,.elf"
        style={{ display: "none" }}
        onChange={(event) => {
          const file = event.currentTarget.files?.[0];
          if (file) {
            void props.onLoadBinary?.(file);
          }
          event.currentTarget.value = "";
        }}
      />
      <button
        type="button"
        class="btn"
        onClick={props.onAssemble}
        disabled={controlsLocked()}
        title="Assemble source (Ctrl+B)"
      >
        Assemble
      </button>
      <button
        type="button"
        class="btn"
        onClick={props.onLoad}
        disabled={controlsLocked()}
        title="Load assembled bytes into engine"
      >
        Load
      </button>
      <Show when={props.onLoadBinary}>
        <button
          type="button"
          class="btn"
          onClick={() => binaryInputRef?.click()}
          disabled={controlsLocked()}
          title="Load a .dol or .elf binary into the engine"
        >
          Open Binary
        </button>
      </Show>
      <div class="control-bar__sep" />
      <div
        class="control-bar__menu"
        onFocusOut={(event) => {
          const nextTarget = event.relatedTarget;
          if (!event.currentTarget.contains(nextTarget as Node | null)) {
            closeMenu();
          }
        }}
      >
        <div class="btn-group">
          <button
            type="button"
            class="btn btn--primary btn-group__main"
            onClick={runSelectedMode}
            disabled={primaryRunDisabled()}
            title={primaryRunTitle()}
          >
            {primaryRunLabel()}
          </button>
          <button
            type="button"
            class="btn btn--primary btn--split-dropdown"
            onClick={() => setMenuOpen((open) => !open)}
            disabled={controlsLocked()}
            title="Choose run mode"
            aria-haspopup="menu"
            aria-expanded={menuOpen() ? "true" : "false"}
          >
            ▾
          </button>
        </div>
        <Show when={menuOpen()}>
          <div class="control-menu" role="menu" aria-label="Run modes">
            <button
              type="button"
              class="control-menu__item"
              onClick={() => selectRunMode("run")}
              role="menuitem"
            >
              ▶ Run
            </button>
            <button
              type="button"
              class="control-menu__item"
              onClick={() => selectRunMode("run-with-emulator")}
              role="menuitem"
            >
              ▶ Run with Emulator
            </button>
          </div>
        </Show>
      </div>
      <Show
        when={showPausedExecutionActions()}
        fallback={(
          <button
            type="button"
            class="btn"
            onClick={props.onStep}
            disabled={stepDisabled()}
            title="Step one instruction (F10)"
          >
            ⤼ Step
          </button>
        )}
      >
        <div class="btn-group control-bar__pause-actions">
          <button
            type="button"
            class="btn"
            onClick={props.onStep}
            disabled={stepDisabled()}
            title="Advance one instruction from the current breakpoint"
          >
            Next Step
          </button>
          <button
            type="button"
            class="btn"
            onClick={props.onNextBranch}
            disabled={stepDisabled() || !props.onNextBranch}
            title="Advance until the next branch instruction"
          >
            Next Branch
          </button>
          <button
            type="button"
            class="btn btn--primary"
            onClick={props.onResume}
            disabled={primaryRunDisabled() || !props.onResume}
            title="Resume execution from the current breakpoint"
          >
            Resume
          </button>
        </div>
      </Show>
      <Show when={props.onStop && props.running}>
        <button type="button" class="btn" onClick={props.onStop} title="Stop">
          ⏸ Stop
        </button>
      </Show>
      <Show when={props.onCloseEmulator}>
        <button
          type="button"
          class="btn"
          onClick={props.onCloseEmulator}
          title="Close Dolphin emulator session"
        >
          ✕ Close Emulator
        </button>
      </Show>
      <Show when={historyVisible()}>
        <div class="control-bar__history" aria-label="Snapshot history">
          <button
            type="button"
            class="btn btn--history"
            onClick={props.onHistoryBack}
            disabled={props.running || !props.onHistoryBack || (props.historyIndex ?? 0) <= 0}
            title="Show older snapshot"
          >
            ⟵
          </button>
          <span class="control-bar__history-label">
            {props.historyAtLive ? "Live" : `Snapshot ${historyIndexLabel()}`}
          </span>
          <button
            type="button"
            class="btn btn--history"
            onClick={props.onHistoryForward}
            disabled={props.running || !props.onHistoryForward || !!props.historyAtLive}
            title="Show newer snapshot"
          >
            ⟶
          </button>
          <Show when={!props.historyAtLive && props.onHistoryLive}>
            <button
              type="button"
              class="btn btn--history-live"
              onClick={props.onHistoryLive}
              disabled={props.running}
              title="Return to the latest emulator snapshot"
            >
              Live
            </button>
          </Show>
        </div>
      </Show>
      <button
        type="button"
        class="btn"
        onClick={props.onReset}
        disabled={controlsLocked()}
        title="Reset CPU state (Ctrl+R)"
      >
        ⟲ Reset
      </button>
      <Show when={props.onManuals}>
        <div class="control-bar__sep" />
        <button
          type="button"
          class="btn"
          onClick={props.onManuals}
          title="Open PPC reference manuals"
        >
          📖 Manuals
        </button>
      </Show>

      <div class="control-bar__status">
        <span>PC&nbsp;<strong>{hex32(props.pc)}</strong></span>
        <span>steps&nbsp;<strong>{props.stepCount.toLocaleString()}</strong></span>
        <span class={pillClass()}>
          {props.halted
            ? haltReasonLabel(props.haltReason)
            : props.running
              ? "running"
              : "ready"}
        </span>
      </div>
    </div>
  );
};
