import { type Component, For, createMemo } from "solid-js";
import type { RegisterSnapshot } from "@ppc-bench/kernel";
import { formatGPR, hex32 } from "../styles/format";
import { Panel } from "../shell/Panel";
import "../styles/registers.css";

export interface RegistersPanelProps {
  registers: RegisterSnapshot;
}

const cr_nibble = (cr: number, n: number): string => {
  const v = (cr >>> (28 - 4 * n)) & 0xf;
  const lt = (v >> 3) & 1, gt = (v >> 2) & 1, eq = (v >> 1) & 1, so = v & 1;
  return (
    (lt ? "L" : "-") + (gt ? "G" : "-") + (eq ? "E" : "-") + (so ? "O" : "-")
  );
};

export const RegistersPanel: Component<RegistersPanelProps> = (props) => {
  const changed = createMemo(() => new Set(props.registers.changed_gpr));
  return (
    <Panel
      title="Registers"
      grow
      actions={<span>GPR + SPR</span>}
    >
      <div class="regs__grid">
        <For each={Array.from({ length: 32 }, (_, i) => i)}>
          {(i) => {
            const v = () => props.registers.gpr[i] ?? 0;
            return (
              <div class={`regs__row${changed().has(i) ? " regs__row--changed" : ""}`}>
                <span class="regs__label">{formatGPR(i)}</span>
                <span class="regs__val">{hex32(v())}</span>
              </div>
            );
          }}
        </For>
      </div>
      <div class="regs__spr">
        <span class="regs__label">PC</span>  <span>{hex32(props.registers.pc)}</span>
        <span class="regs__label">LR</span>  <span>{hex32(props.registers.lr)}</span>
        <span class="regs__label">CTR</span> <span>{hex32(props.registers.ctr)}</span>
        <span class="regs__label">XER</span> <span>{hex32(props.registers.xer)}</span>
        <span class="regs__label">MSR</span> <span>{hex32(props.registers.msr)}</span>
        <span class="regs__label">CR</span>
        <span>{hex32(props.registers.cr)}</span>
        <For each={Array.from({ length: 8 }, (_, i) => i)}>
          {(n) => (
            <>
              <span />
              <span class="regs__cr-field">CR{n}  {cr_nibble(props.registers.cr, n)}</span>
            </>
          )}
        </For>
        <hr class="regs__section-divider" />
        <For each={props.registers.gqr ?? []}>
          {(v, i) => (
            <>
              <span class="regs__label">GQR{i()}</span>
              <span>{hex32(v)}</span>
            </>
          )}
        </For>
      </div>
    </Panel>
  );
};
