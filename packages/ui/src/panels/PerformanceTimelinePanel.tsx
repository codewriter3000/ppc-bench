import { type Component, For, Show, createMemo } from "solid-js";
import { hex32 } from "../styles/format";
import { Panel } from "../shell/Panel";
import "../styles/panels-misc.css";

export interface PerformanceSample {
  id: number;
  source: "engine" | "emulator";
  label: string;
  timestampMs: number;
  elapsedMs: number;
  stepCount: number;
  deltaSteps: number | null;
  instructionsPerSecond: number | null;
  pc: number;
  note?: string;
}

export interface PerformanceTimelinePanelProps {
  samples: readonly PerformanceSample[];
  onJump?: (addr: number) => void;
}

const formatIps = (value: number | null) => value == null ? "—" : `${Math.round(value).toLocaleString()} ips`;

export const PerformanceTimelinePanel: Component<PerformanceTimelinePanelProps> = (props) => {
  const visibleSamples = createMemo(() => props.samples.slice(-48));
  const recentRows = createMemo(() => [...props.samples].slice(-6).reverse());
  const maxIps = createMemo(() => Math.max(1, ...visibleSamples().map((sample) => sample.instructionsPerSecond ?? 0)));

  return (
    <Panel
      title="Performance"
      grow
      actions={<span>{props.samples.length} samples</span>}
    >
      <Show
        when={props.samples.length > 0}
        fallback={<div class="perf__empty">Run, step, or pause execution to record throughput samples.</div>}
      >
        <div class="perf__chart" role="img" aria-label="Instruction throughput timeline">
          <For each={visibleSamples()}>
            {(sample) => {
              const height = () => sample.instructionsPerSecond == null
                ? 10
                : Math.max(10, (sample.instructionsPerSecond / maxIps()) * 100);
              return (
                <button
                  type="button"
                  class={`perf__bar perf__bar--${sample.source}${sample.instructionsPerSecond == null ? " perf__bar--unknown" : ""}`}
                  style={{ height: `${height()}%` }}
                  onClick={() => props.onJump?.(sample.pc >>> 0)}
                  title={`${sample.label}: ${formatIps(sample.instructionsPerSecond)} at ${hex32(sample.pc)}${sample.note ? ` — ${sample.note}` : ""}`}
                />
              );
            }}
          </For>
        </div>
        <div class="perf__head">
          <span>sample</span>
          <span>elapsed</span>
          <span>ips</span>
          <span>pc</span>
          <span>note</span>
        </div>
        <For each={recentRows()}>
          {(sample) => (
            <div class="perf__row" onClick={() => props.onJump?.(sample.pc >>> 0)}>
              <span class="perf__sample-label">{sample.label}</span>
              <span>{sample.elapsedMs.toFixed(1)} ms</span>
              <span>{formatIps(sample.instructionsPerSecond)}</span>
              <span class="perf__pc">{hex32(sample.pc)}</span>
              <span class="perf__note">{sample.note ?? sample.source}</span>
            </div>
          )}
        </For>
      </Show>
    </Panel>
  );
};