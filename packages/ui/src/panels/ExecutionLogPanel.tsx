import { type Component, For, Show } from "solid-js";
import type { TraceEntry } from "@ppc-bench/kernel";
import { hex32 } from "../styles/format";
import { Panel } from "../shell/Panel";
import "../styles/panels-misc.css";

export interface ExecutionLogPanelProps {
  trace: readonly TraceEntry[];
  onJump?: (addr: number) => void;
}

export const ExecutionLogPanel: Component<ExecutionLogPanelProps> = (props) => {
  return (
    <Panel
      title="Execution Log"
      grow
      actions={<span>{props.trace.length} entries</span>}
    >
      <Show
        when={props.trace.length > 0}
        fallback={<div class="exec-log__empty">Run or step to record trace entries.</div>}
      >
        <div class="exec-log__head">
          <span>step</span>
          <span>pc</span>
          <span>raw</span>
          <span>mnem</span>
        </div>
        <For each={props.trace}>
          {(entry) => (
            <div class="exec-log__row" onClick={() => props.onJump?.(entry.pc >>> 0)}>
              <span class="exec-log__addr">{entry.step}</span>
              <span>{hex32(entry.pc)}</span>
              <span class="exec-log__addr">{hex32(entry.raw)}</span>
              <span class="exec-log__mnem">{entry.mnemonic}</span>
            </div>
          )}
        </For>
      </Show>
    </Panel>
  );
};
