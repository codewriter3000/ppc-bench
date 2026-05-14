import { type Component, For, Show, createEffect, createMemo } from "solid-js";
import type { DisasmLine } from "@ppc-bench/kernel";
import { hex32 } from "../styles/format";
import { Panel } from "../shell/Panel";
import "../styles/disassembly.css";

export interface DisassemblyPanelProps {
  lines: readonly DisasmLine[];
  pc: number;
  breakpoints: readonly number[];
  symbols?: ReadonlyArray<readonly [string, number]>;
  onToggleBreakpoint: (addr: number) => void;
  /** When true (default), keeps the PC row scrolled into view. */
  followPc?: boolean;
}

export const DisassemblyPanel: Component<DisassemblyPanelProps> = (props) => {
  const bpSet = createMemo(() => new Set(props.breakpoints.map((a) => a >>> 0)));
  const labelByAddr = createMemo(() => {
    const m = new Map<number, string>();
    for (const [name, addr] of props.symbols ?? []) {
      const a = addr >>> 0;
      if (!m.has(a)) m.set(a, name);
    }
    return m;
  });

  let pcRowEl: HTMLDivElement | undefined;
  createEffect(() => {
    void props.pc;
    if (props.followPc === false) return;
    pcRowEl?.scrollIntoView({ block: "nearest", behavior: "smooth" });
  });

  return (
    <Panel
      title="Disassembly"
      grow
      actions={<span>{props.lines.length} insn</span>}
    >
      <Show
        when={props.lines.length > 0}
        fallback={<div class="disasm__empty">No program loaded. Assemble &amp; load to see disassembly.</div>}
      >
        <div class="disasm__list">
          <For each={props.lines}>
            {(line) => {
              const addr = line.address >>> 0;
              const current = () => (props.pc >>> 0) === addr;
              const bp = () => bpSet().has(addr);
              const label = () => labelByAddr().get(addr);
              return (
                <>
                  <Show when={label()}>
                    <div class="disasm__label">{label()}:</div>
                  </Show>
                  <div
                    ref={(el) => { if (current()) pcRowEl = el; }}
                    data-disasm-addr={addr}
                    class={`disasm__row${current() ? " disasm__row--current" : ""}${bp() ? " disasm__row--bp" : ""}`}
                    onClick={() => props.onToggleBreakpoint(addr)}
                    title="Click to toggle breakpoint"
                  >
                    <span class="disasm__bp-dot">{bp() ? "●" : ""}</span>
                    <span class="disasm__addr">{hex32(addr)}</span>
                    <span class="disasm__raw">{hex32(line.raw)}</span>
                    <span class="disasm__mnem">{line.mnemonic}</span>
                    <span>{line.operands}</span>
                  </div>
                </>
              );
            }}
          </For>
        </div>
      </Show>
    </Panel>
  );
};
