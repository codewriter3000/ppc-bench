import { type Component, For, Show } from "solid-js";
import type { CallFrame } from "@ppc-bench/kernel";
import { hex32 } from "../styles/format";
import { Panel } from "../shell/Panel";
import "../styles/panels-misc.css";

export interface CallStackPanelProps {
  frames: readonly CallFrame[];
  onJump?: (addr: number) => void;
}

export const CallStackPanel: Component<CallStackPanelProps> = (props) => {
  return (
    <Panel
      title="Call Stack"
      grow
      actions={<span>{props.frames.length}</span>}
    >
      <Show
        when={props.frames.length > 0}
        fallback={<div class="call-stack__empty">(empty)</div>}
      >
        <For each={[...props.frames].reverse()}>
          {(frame, i) => (
            <div
              class="call-stack__row"
              onClick={() => props.onJump?.(frame.return_to >>> 0)}
              title="Jump to return address"
            >
              <span class="call-stack__frame">{props.frames.length - 1 - i()}</span>
              <span class="call-stack__addr">{hex32(frame.call_site)}</span>
              <span class="call-stack__ret">{hex32(frame.return_to)}</span>
              <span class="call-stack__sym">{frame.symbol ?? "—"}</span>
            </div>
          )}
        </For>
      </Show>
    </Panel>
  );
};
