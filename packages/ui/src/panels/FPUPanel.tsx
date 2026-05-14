import { type Component, For, createMemo } from "solid-js";
import type { FPUSnapshot } from "@ppc-bench/kernel";
import { formatFPR, hex32 } from "../styles/format";
import { Panel } from "../shell/Panel";
import "../styles/fpu.css";

export interface FPUPanelProps {
  fpu: FPUSnapshot;
}

const fmt = (v: number): string => {
  if (Number.isNaN(v)) return "NaN";
  if (!Number.isFinite(v)) return v > 0 ? "+Inf" : "-Inf";
  if (v === 0) return Object.is(v, -0) ? "-0" : "0";
  const abs = Math.abs(v);
  if (abs >= 1e6 || abs < 1e-3) return v.toExponential(6);
  return v.toPrecision(8);
};

export const FPUPanel: Component<FPUPanelProps> = (props) => {
  const changed = createMemo(() => new Set(props.fpu.changed_fpr));
  return (
    <Panel
      title="FPU (paired singles)"
      grow
      actions={<span>FPSCR {hex32(props.fpu.fpscr)}</span>}
    >
      <div class="fpu__head">
        <span>reg</span>
        <span>ps0</span>
        <span>ps1</span>
      </div>
      <div class="fpu__list">
        <For each={Array.from({ length: 32 }, (_, i) => i)}>
          {(i) => {
            const pair = () => props.fpu.fpr[i] ?? [0, 0];
            return (
              <div class={`fpu__row${changed().has(i) ? " fpu__row--changed" : ""}`}>
                <span class="fpu__label">{formatFPR(i)}</span>
                <span>{fmt(pair()[0])}</span>
                <span>{fmt(pair()[1])}</span>
              </div>
            );
          }}
        </For>
      </div>
      <div class="fpu__foot">
        <span class="fpu__label">FPSCR</span>
        <span>{hex32(props.fpu.fpscr)}</span>
      </div>
    </Panel>
  );
};
