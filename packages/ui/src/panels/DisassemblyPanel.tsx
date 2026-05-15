import { type Component, For, Show, createEffect, createMemo, createSignal, onCleanup, onMount } from "solid-js";
import type { DisasmLine } from "@ppc-bench/kernel";
import { hex32 } from "../styles/format";
import { Panel } from "../shell/Panel";
import "../styles/disassembly.css";

const BRANCH_TARGET_TOKEN_RE = /0x[0-9a-f]+(?!.*0x[0-9a-f])/i;
const MEM1_SIZE = 16 * 1024 * 1024;
const CACHED_MEM1_BASE = 0x8000_0000;
const UNCACHED_MEM1_BASE = 0xC000_0000;

type AddressColumnMode = "address" | "pc";

const mem1Offset = (addr: number): number | null => {
  const value = addr >>> 0;
  if (value < MEM1_SIZE) {
    return value;
  }
  if (value >= CACHED_MEM1_BASE && value < CACHED_MEM1_BASE + MEM1_SIZE) {
    return (value - CACHED_MEM1_BASE) >>> 0;
  }
  if (value >= UNCACHED_MEM1_BASE && value < UNCACHED_MEM1_BASE + MEM1_SIZE) {
    return (value - UNCACHED_MEM1_BASE) >>> 0;
  }
  return null;
};

const formatColumnAddress = (addr: number, mode: AddressColumnMode) => {
  if (mode === "pc") {
    const offset = mem1Offset(addr);
    if (offset != null) {
      return hex32(offset);
    }
  }
  return hex32(addr);
};

const getStaticBranchTarget = (line: DisasmLine): number | null => {
  const raw = line.raw >>> 0;
  const opcode = raw >>> 26;

  if (opcode === 18) {
    const displacement = ((raw & 0x03ff_fffc) << 6) >> 6;
    return (raw & 0x2) !== 0 ? displacement >>> 0 : (line.address + displacement) >>> 0;
  }

  if (opcode === 16) {
    const displacement = ((raw & 0x0000_fffc) << 16) >> 16;
    return (raw & 0x2) !== 0 ? displacement >>> 0 : (line.address + displacement) >>> 0;
  }

  return null;
};

export interface DisassemblyJumpRequest {
  addr: number;
  token: number;
}

export interface DisassemblyPanelProps {
  lines: readonly DisasmLine[];
  pc: number;
  breakpoints: readonly number[];
  symbols?: ReadonlyArray<readonly [string, number]>;
  onToggleBreakpoint: (addr: number) => void;
  onJump?: (addr: number) => void;
  jumpRequest?: DisassemblyJumpRequest | null;
  lineLimit?: number;
  /** When true (default), keeps the PC row scrolled into view. */
  followPc?: boolean;
}

export const DisassemblyPanel: Component<DisassemblyPanelProps> = (props) => {
  const [windowStart, setWindowStart] = createSignal(0);
  const [ctrlHeld, setCtrlHeld] = createSignal(false);
  const [pageInput, setPageInput] = createSignal("1");
  const [addressColumnMode, setAddressColumnMode] = createSignal<AddressColumnMode>("address");
  const lineLimit = createMemo(() => Math.max(64, props.lineLimit ?? 1000));
  const bpSet = createMemo(() => new Set(props.breakpoints.map((a) => a >>> 0)));
  const addrToIndex = createMemo(() => {
    const map = new Map<number, number>();
    props.lines.forEach((line, index) => {
      map.set(line.address >>> 0, index);
    });
    return map;
  });
  const labelByAddr = createMemo(() => {
    const m = new Map<number, string>();
    for (const [name, addr] of props.symbols ?? []) {
      const a = addr >>> 0;
      if (!m.has(a)) m.set(a, name);
    }
    return m;
  });
  const resolveLineAddress = (addr: number) => {
    const exact = addr >>> 0;
    const indexMap = addrToIndex();
    if (indexMap.has(exact)) {
      return exact;
    }

    const offset = mem1Offset(exact);
    if (offset == null) {
      return null;
    }

    const candidates = [offset, (CACHED_MEM1_BASE + offset) >>> 0, (UNCACHED_MEM1_BASE + offset) >>> 0];
    for (const candidate of candidates) {
      if (indexMap.has(candidate >>> 0)) {
        return candidate >>> 0;
      }
    }

    return null;
  };
  const resolvedPc = createMemo(() => resolveLineAddress(props.pc));
  const addressColumnLabel = createMemo(() => addressColumnMode() === "pc" ? "PC" : "Address");
  const clampWindowStart = (nextStart: number) => {
    const total = props.lines.length;
    const limit = lineLimit();
    if (total <= limit) {
      return 0;
    }

    return Math.max(0, Math.min(total - limit, nextStart));
  };
  const totalPages = createMemo(() => Math.max(1, Math.ceil(props.lines.length / lineLimit())));
  const pageStarts = createMemo(() => {
    const total = props.lines.length;
    const limit = lineLimit();
    const maxStart = Math.max(0, total - limit);
    return Array.from({ length: totalPages() }, (_, index) => Math.min(index * limit, maxStart));
  });
  const currentPage = createMemo(() => {
    const start = clampWindowStart(windowStart());
    const index = pageStarts().indexOf(start);
    return index >= 0 ? index + 1 : 1;
  });
  const goToPage = (page: number) => {
    const normalized = Math.max(1, Math.min(totalPages(), Math.trunc(page) || 1));
    setWindowStart(pageStarts()[normalized - 1] ?? 0);
  };
  const centerWindowOnAddr = (addr: number) => {
    const resolvedAddr = resolveLineAddress(addr);
    if (resolvedAddr == null) {
      return false;
    }

    const index = addrToIndex().get(resolvedAddr);
    if (index == null) {
      return false;
    }

    goToPage(Math.floor(index / lineLimit()) + 1);
    return true;
  };
  const visibleWindow = createMemo(() => {
    const total = props.lines.length;
    const start = clampWindowStart(windowStart());
    const end = Math.min(total, start + lineLimit());
    return {
      start,
      end,
      lines: props.lines.slice(start, end),
    };
  });

  createEffect(() => {
    const pc = resolvedPc();
    if (props.followPc === false) return;
    if (pc == null) return;
    centerWindowOnAddr(pc);

    requestAnimationFrame(() => {
      document
        .querySelector<HTMLElement>(`[data-disasm-addr="${pc}"]`)
        ?.scrollIntoView({ block: "nearest", behavior: "smooth" });
    });
  });

  createEffect(() => {
    const request = props.jumpRequest;
    if (!request) {
      return;
    }

    void request.token;
    const resolvedAddr = resolveLineAddress(request.addr);
    if (resolvedAddr == null || !centerWindowOnAddr(resolvedAddr)) {
      return;
    }

    requestAnimationFrame(() => {
      document
        .querySelector<HTMLElement>(`[data-disasm-addr="${resolvedAddr}"]`)
        ?.scrollIntoView({ block: "center", behavior: "smooth" });
    });
  });

  createEffect(() => {
    void props.lines.length;
    setWindowStart((current) => clampWindowStart(current));
  });

  createEffect(() => {
    setPageInput(String(currentPage()));
  });

  onMount(() => {
    const syncCtrl = (event: KeyboardEvent) => setCtrlHeld(event.ctrlKey);
    const clearCtrl = () => setCtrlHeld(false);
    window.addEventListener("keydown", syncCtrl);
    window.addEventListener("keyup", syncCtrl);
    window.addEventListener("blur", clearCtrl);
    onCleanup(() => {
      window.removeEventListener("keydown", syncCtrl);
      window.removeEventListener("keyup", syncCtrl);
      window.removeEventListener("blur", clearCtrl);
    });
  });

  const canPageBackward = () => visibleWindow().start > 0;
  const canPageForward = () => visibleWindow().end < props.lines.length;
  const commitPageInput = () => {
    const parsed = Number.parseInt(pageInput().trim(), 10);
    if (!Number.isFinite(parsed)) {
      setPageInput(String(currentPage()));
      return;
    }

    goToPage(parsed);
    setPageInput(String(currentPage()));
  };

  return (
    <Panel
      title="Disassembly"
      grow
      actions={(
        <div class="disasm__actions">
          <button
            type="button"
            class={`disasm__toggle${addressColumnMode() === "pc" ? " disasm__toggle--active" : ""}`}
            onClick={() => setAddressColumnMode((current) => current === "address" ? "pc" : "address")}
            title={addressColumnMode() === "address"
              ? "Show the first disassembly column in program-counter space"
              : "Show the first disassembly column in loaded-address space"}
          >
            {addressColumnMode() === "address" ? "Show PC" : "Show Address"}
          </button>
          <button
            type="button"
            class="disasm__nav"
            disabled={!canPageBackward()}
            onClick={() => goToPage(currentPage() - 1)}
            title="Show the previous disassembly window"
          >
            ⟵
          </button>
          <label class="disasm__page" title={`Jump to a disassembly page between 1 and ${totalPages()}`}>
            <span>Page</span>
            <input
              type="text"
              inputMode="numeric"
              pattern="[0-9]*"
              class="disasm__page-input"
              value={pageInput()}
              onInput={(event) => setPageInput(event.currentTarget.value)}
              onBlur={() => commitPageInput()}
              onKeyDown={(event) => {
                if (event.key === "Enter") {
                  event.preventDefault();
                  commitPageInput();
                  event.currentTarget.blur();
                }
              }}
              aria-label="Current disassembly page"
            />
            <span>of {totalPages()}</span>
          </label>
          <span class="disasm__range">{visibleWindow().start + 1}-{visibleWindow().end} / {props.lines.length}</span>
          <button
            type="button"
            class="disasm__nav"
            disabled={!canPageForward()}
            onClick={() => goToPage(currentPage() + 1)}
            title="Show the next disassembly window"
          >
            ⟶
          </button>
        </div>
      )}
    >
      <Show
        when={props.lines.length > 0}
        fallback={<div class="disasm__empty">No program loaded. Assemble &amp; load to see disassembly.</div>}
      >
        <div class={`disasm__list${ctrlHeld() ? " disasm__list--ctrl-held" : ""}`}>
          <div class="disasm__head">
            <span aria-hidden="true" />
            <span>{addressColumnLabel()}</span>
            <span>Raw</span>
            <span>Mnemonic</span>
            <span>Operands</span>
          </div>
          <For each={visibleWindow().lines}>
            {(line) => {
              const addr = line.address >>> 0;
              const current = () => resolvedPc() === addr;
              const bp = () => bpSet().has(addr);
              const label = () => line.label ?? labelByAddr().get(addr);
              const branchTarget = () => getStaticBranchTarget(line);
              const branchTargetToken = () => BRANCH_TARGET_TOKEN_RE.exec(line.operands);
              const canJumpToBranch = () => {
                const target = branchTarget();
                return target != null && branchTargetToken() != null && addrToIndex().has(target >>> 0) && !!props.onJump;
              };
              const renderOperands = () => {
                const match = branchTargetToken();
                const target = branchTarget();
                if (!match || target == null || !canJumpToBranch()) {
                  return line.operands;
                }

                const start = match.index ?? 0;
                const end = start + match[0].length;
                return (
                  <>
                    <span>{line.operands.slice(0, start)}</span>
                    <button
                      type="button"
                      class="disasm__branch-target"
                      title={ctrlHeld() ? `Ctrl+Click to jump to ${hex32(target)}` : `Hold Ctrl to jump to ${hex32(target)}`}
                      onClick={(event) => {
                        event.stopPropagation();
                        if (!event.ctrlKey) {
                          return;
                        }

                        props.onJump?.(target);
                      }}
                    >
                      {match[0]}
                    </button>
                    <span>{line.operands.slice(end)}</span>
                  </>
                );
              };
              return (
                <>
                  <Show when={label()}>
                    <div class="disasm__label">{label()}:</div>
                  </Show>
                  <div
                    data-disasm-addr={addr}
                    class={`disasm__row${current() ? " disasm__row--current" : ""}${bp() ? " disasm__row--bp" : ""}`}
                    onClick={() => props.onToggleBreakpoint(addr)}
                    title="Click to toggle breakpoint"
                  >
                    <span class="disasm__bp-dot">{bp() ? "●" : ""}</span>
                    <span class="disasm__addr">{formatColumnAddress(addr, addressColumnMode())}</span>
                    <span class="disasm__raw">{hex32(line.raw)}</span>
                    <span class="disasm__mnem">{line.mnemonic}</span>
                    <span>{renderOperands()}</span>
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
