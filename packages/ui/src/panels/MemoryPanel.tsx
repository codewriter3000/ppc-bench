import { type Component, For, Show, createSignal, createResource, createMemo } from "solid-js";
import type { MemoryWrite } from "@ppc-bench/kernel";
import { asciiOf, hex32, hex8 } from "../styles/format";
import { Panel } from "../shell/Panel";
import "../styles/memory.css";

export interface MemoryPanelProps {
  onReadMemory: (addr: number, len: number) => Promise<Uint8Array>;
  lastWrites: readonly MemoryWrite[];
  initialAddr?: number;
  bytesPerRow?: number;
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

const parseLen = (s: string): number | null => {
  const n = parseInt(s.trim(), 10);
  return Number.isFinite(n) && n > 0 && n <= 65536 ? n | 0 : null;
};

export const MemoryPanel: Component<MemoryPanelProps> = (props) => {
  const bpr = () => props.bytesPerRow ?? 16;
  const [addr, setAddr] = createSignal(props.initialAddr ?? 0x8000_0000);
  const [addrInput, setAddrInput] = createSignal(hex32(props.initialAddr ?? 0x8000_0000));
  const [len, setLen] = createSignal(256);
  const [lenInput, setLenInput] = createSignal("256");
  const rows = createMemo(() => Math.ceil(len() / bpr()));

  // Use a string key so === comparison works — a plain object `{}` always fails ===
  // and would cause the resource to constantly re-fetch.
  const resourceKey = createMemo(() => {
    const ws = props.lastWrites;
    return `${addr()}:${len()}:${ws.map(w => `${w.addr}+${w.size}`).join(",")}`;
  });
  const [bytes] = createResource(resourceKey, () => props.onReadMemory(addr(), len()));

  // Reactive write-highlight set — updates when lastWrites changes
  const writeSet = createMemo(() => {
    const s = new Set<number>();
    for (const w of props.lastWrites)
      for (let i = 0; i < w.size; i++) s.add((w.addr + i) >>> 0);
    return s;
  });

  const onGo = () => {
    const a = parseAddr(addrInput());
    if (a != null) setAddr(a);
  };

  const onLenCommit = () => {
    const n = parseLen(lenInput());
    if (n != null) { setLen(n); setLenInput(String(n)); }
    else setLenInput(String(len()));
  };

  return (
    <Panel title="Memory" grow>
      <div class="memory__toolbar">
        <span class="memory__addr-label">addr</span>
        <input
          class="memory__input"
          value={addrInput()}
          onInput={(e) => setAddrInput(e.currentTarget.value)}
          onKeyDown={(e) => e.key === "Enter" && onGo()}
          spellcheck={false}
        />
        <button type="button" class="btn" onClick={onGo}>Go</button>
        <span class="memory__addr-label">bytes</span>
        <input
          class="memory__input memory__input--short"
          value={lenInput()}
          onInput={(e) => setLenInput(e.currentTarget.value)}
          onKeyDown={(e) => e.key === "Enter" && onLenCommit()}
          onBlur={onLenCommit}
          spellcheck={false}
        />
      </div>
      <Show when={bytes()} fallback={<div class="memory__empty">Loading…</div>}>
        {(data) => (
          <For each={Array.from({ length: rows() }, (_, r) => r)}>
            {(r) => {
              // createMemo inside a <For> item is valid — each item has its own
              // reactive owner, so these memos update when addr/data change.
              const base = createMemo(() => (addr() + r * bpr()) >>> 0);
              const chunk = createMemo(() =>
                Array.from(data().slice(r * bpr(), r * bpr() + bpr()))
              );
              return (
                <div class="memory__row">
                  <span class="memory__addr">{hex32(base())}</span>
                  <span>
                    <For each={chunk()}>
                      {(b, i) => (
                        <span class={`memory__byte${writeSet().has((base() + i()) >>> 0) ? " memory__byte--written" : ""}`}>
                          {hex8(b)}{i() < bpr() - 1 ? " " : ""}
                        </span>
                      )}
                    </For>
                  </span>
                  <span>{chunk().map(b => asciiOf(b)).join("")}</span>
                </div>
              );
            }}
          </For>
        )}
      </Show>
    </Panel>
  );
};
