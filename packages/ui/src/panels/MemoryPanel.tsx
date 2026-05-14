import { type Component, For, Show, createSignal, createResource } from "solid-js";
import type { MemoryWrite } from "@ppc-bench/kernel";
import { asciiOf, hex32, hex8 } from "../styles/format";
import { Panel } from "../shell/Panel";
import "../styles/memory.css";

export interface MemoryPanelProps {
  onReadMemory: (addr: number, len: number) => Promise<Uint8Array>;
  lastWrites: readonly MemoryWrite[];
  initialAddr?: number;
  bytesPerRow?: number;
  rows?: number;
}

const parseAddr = (s: string): number | null => {
  const t = s.trim();
  if (!t) return null;
  if (t.startsWith("0x") || t.startsWith("0X")) {
    const n = parseInt(t.slice(2), 16);
    return Number.isFinite(n) ? n >>> 0 : null;
  }
  const n = parseInt(t, 10);
  return Number.isFinite(n) ? n >>> 0 : null;
};

export const MemoryPanel: Component<MemoryPanelProps> = (props) => {
  const bpr = () => props.bytesPerRow ?? 16;
  const rows = () => props.rows ?? 16;
  const [addr, setAddr] = createSignal(props.initialAddr ?? 0x8000_0000);
  const [input, setInput] = createSignal(hex32(props.initialAddr ?? 0x8000_0000));
  const len = () => bpr() * rows();
  const [bytes] = createResource(
    () => ({ addr: addr(), len: len(), ts: props.lastWrites }),
    async (k) => await props.onReadMemory(k.addr, k.len),
  );

  const writeRanges = () => {
    const s = new Set<number>();
    for (const w of props.lastWrites) {
      for (let i = 0; i < w.size; i++) s.add((w.addr + i) >>> 0);
    }
    return s;
  };

  const onGo = () => {
    const a = parseAddr(input());
    if (a != null) setAddr(a);
  };

  return (
    <Panel
      title="Memory"
      grow
      actions={<span style="color:var(--color-text-muted)">{len()} bytes</span>}
    >
      <div class="memory__toolbar">
        <span class="memory__addr-label">addr</span>
        <input
          class="memory__input"
          value={input()}
          onInput={(e) => setInput(e.currentTarget.value)}
          onKeyDown={(e) => e.key === "Enter" && onGo()}
          spellcheck={false}
        />
        <button type="button" class="btn" onClick={onGo}>Go</button>
      </div>
      <Show
        when={bytes()}
        fallback={<div class="memory__empty">Loading…</div>}
      >
        {(data) => {
          const wr = writeRanges();
          return (
            <For each={Array.from({ length: rows() }, (_, r) => r)}>
              {(r) => {
                const base = (addr() + r * bpr()) >>> 0;
                const chunk = () => data().slice(r * bpr(), r * bpr() + bpr());
                return (
                  <div class="memory__row">
                    <span class="memory__addr">{hex32(base)}</span>
                    <span>
                      {Array.from(chunk()).map((b, i) => {
                        const a = (base + i) >>> 0;
                        return (
                          <span
                            class={`memory__byte${wr.has(a) ? " memory__byte--written" : ""}`}
                          >
                            {hex8(b)}{i < bpr() - 1 ? " " : ""}
                          </span>
                        );
                      })}
                    </span>
                    <span>{Array.from(chunk()).map((b) => asciiOf(b)).join("")}</span>
                  </div>
                );
              }}
            </For>
          );
        }}
      </Show>
    </Panel>
  );
};
