import { type Component, For, Show, createMemo } from "solid-js";
import { hex32 } from "../styles/format";
import { Panel } from "../shell/Panel";
import "../styles/panels-misc.css";

export interface BreakpointsPanelProps {
  breakpoints: readonly number[];
  symbols?: ReadonlyArray<readonly [string, number]>;
  onClear: (addr: number) => void;
  onJump?: (addr: number) => void;
}

export const BreakpointsPanel: Component<BreakpointsPanelProps> = (props) => {
  const symByAddr = createMemo(() => {
    const m = new Map<number, string>();
    for (const [name, addr] of props.symbols ?? []) {
      const a = addr >>> 0;
      if (!m.has(a)) m.set(a, name);
    }
    return m;
  });
  return (
    <Panel
      title="Breakpoints"
      grow
      actions={<span>{props.breakpoints.length}</span>}
    >
      <Show
        when={props.breakpoints.length > 0}
        fallback={<div class="bps__empty">Click a disassembly row to add a breakpoint.</div>}
      >
        <For each={props.breakpoints}>
          {(addr) => (
            <div class="bps__row">
              <span class="bps__dot">●</span>
              <span
                class="bps__addr"
                style="cursor:pointer"
                onClick={() => props.onJump?.(addr >>> 0)}
              >
                {hex32(addr)}
              </span>
              <span class="bps__sym">{symByAddr().get(addr >>> 0) ?? ""}</span>
              <button
                type="button"
                class="btn"
                style="min-height:22px;padding:0 8px;font-size:0.72rem;"
                onClick={() => props.onClear(addr >>> 0)}
              >
                Clear
              </button>
            </div>
          )}
        </For>
      </Show>
    </Panel>
  );
};
