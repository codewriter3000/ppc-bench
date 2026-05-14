import { type Component, Show } from "solid-js";
import type { HaltReason } from "@ppc-bench/kernel";
import { haltReasonLabel } from "@ppc-bench/kernel";
import { hex32 } from "../styles/format";
import "../styles/control-bar.css";

export interface ControlBarProps {
  pc: number;
  stepCount: number;
  halted: boolean;
  haltReason: HaltReason;
  running: boolean;
  onAssemble: () => void;
  onLoad: () => void;
  onStep: () => void;
  onRun: () => void;
  onStop?: () => void;
  onReset: () => void;
  onManuals?: () => void;
}

export const ControlBar: Component<ControlBarProps> = (props) => {
  const pillClass = () =>
    props.halted ? "pill pill--halted" : props.running ? "pill pill--running" : "pill pill--ready";
  return (
    <div class="control-bar" role="toolbar" aria-label="PPC-Bench controls">
      <button
        type="button"
        class="btn"
        onClick={props.onAssemble}
        disabled={props.running}
        title="Assemble source (Ctrl+B)"
      >
        Assemble
      </button>
      <button
        type="button"
        class="btn"
        onClick={props.onLoad}
        disabled={props.running}
        title="Load assembled bytes into engine"
      >
        Load
      </button>
      <div class="control-bar__sep" />
      <button
        type="button"
        class="btn btn--primary"
        onClick={props.onRun}
        disabled={props.running || props.halted}
        title="Run until halt (F5)"
      >
        ▶ Run
      </button>
      <button
        type="button"
        class="btn"
        onClick={props.onStep}
        disabled={props.running || props.halted}
        title="Step one instruction (F10)"
      >
        ⤼ Step
      </button>
      <Show when={props.onStop && props.running}>
        <button type="button" class="btn" onClick={props.onStop} title="Stop">
          ⏸ Stop
        </button>
      </Show>
      <button
        type="button"
        class="btn"
        onClick={props.onReset}
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
