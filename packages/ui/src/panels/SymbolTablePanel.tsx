import { type Component, For, Show, createMemo } from "solid-js";
import { hex32 } from "../styles/format";
import { Panel } from "../shell/Panel";
import "../styles/panels-misc.css";

export interface SymbolTablePanelProps {
  symbols: ReadonlyArray<readonly [string, number]>;
  onJump?: (addr: number) => void;
}

export const SymbolTablePanel: Component<SymbolTablePanelProps> = (props) => {
  const sorted = createMemo(() =>
    [...props.symbols].sort((a, b) => (a[1] >>> 0) - (b[1] >>> 0)),
  );
  return (
    <Panel
      title="Symbols"
      grow
      actions={<span>{props.symbols.length}</span>}
    >
      <Show
        when={sorted().length > 0}
        fallback={<div class="syms__empty">No symbols. Define labels in your source to populate this table.</div>}
      >
        <For each={sorted()}>
          {([name, addr]) => (
            <div class="syms__row" onClick={() => props.onJump?.(addr >>> 0)}>
              <span class="syms__addr">{hex32(addr)}</span>
              <span class="syms__name">{name}</span>
            </div>
          )}
        </For>
      </Show>
    </Panel>
  );
};
