import { type Component, For, Show, createMemo } from "solid-js";
import { hex32 } from "../styles/format";
import { Panel } from "../shell/Panel";
import "../styles/panels-misc.css";

export interface BreakpointsPanelProps {
  breakpoints: readonly number[];
  symbols?: ReadonlyArray<readonly [string, number]>;
  onClear: (addr: number) => void;
  onJump?: (addr: number) => void;
  enableConditions?: boolean;
  conditionValues?: Readonly<Record<number, string>>;
  conditionStatuses?: Readonly<Record<number, BreakpointConditionStatus | undefined>>;
  onConditionChange?: (addr: number, value: string) => void;
}

export interface BreakpointConditionStatus {
  message: string;
  tone: "muted" | "error" | "success";
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
          {(addr) => {
            const conditionValue = () => props.conditionValues?.[addr >>> 0] ?? "";
            const conditionStatus = () => props.conditionStatuses?.[addr >>> 0];
            return (
            <div class="bps__row">
              <div class="bps__main">
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
              <Show when={props.enableConditions && props.onConditionChange}>
                <div class="bps__condition">
                  <input
                    class="bps__condition-input"
                    value={conditionValue()}
                    onInput={(event) => props.onConditionChange?.(addr >>> 0, event.currentTarget.value)}
                    placeholder="pc == 0x80000020"
                    spellcheck={false}
                    title="Conditional breakpoint syntax: register/value comparison, for example r3 == 42 or sp >= 0x80001000"
                  />
                  <Show when={conditionStatus()}>
                    {(status) => (
                      <span class={`bps__condition-note bps__condition-note--${status().tone}`}>
                        {status().message}
                      </span>
                    )}
                  </Show>
                </div>
              </Show>
            </div>
            );
          }}
        </For>
      </Show>
    </Panel>
  );
};
