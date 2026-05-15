import { type Component, For, Show, createEffect, createMemo, createSignal } from "solid-js";
import type { TraceEntry } from "@ppc-bench/kernel";
import { hex32 } from "../styles/format";
import { Panel } from "../shell/Panel";
import type { PerformanceSample } from "./PerformanceTimelinePanel";
import "../styles/panels-misc.css";

export interface RuntimeErrorReport {
  id: number;
  title: string;
  summary: string;
  source: string;
  pc: number;
  stepCount: number;
  assembledSourceLocation?: {
    line: number;
    text: string;
    startAddr: number;
    endAddr: number;
  } | null;
  affectedAddress?: number | null;
  faultingInstruction?: {
    step?: number | null;
    raw: number;
    mnemonic: string;
    operands: string;
  } | null;
  previousSteps: readonly TraceEntry[];
}

export interface DiagnosticsPanelProps {
  performanceSamples: readonly PerformanceSample[];
  runtimeError?: RuntimeErrorReport | null;
  errorContextSteps?: number;
  onJump?: (addr: number) => void;
  onClearError?: () => void;
}

type DiagnosticsTab = "performance" | "errors";

const formatIps = (value: number | null) => value == null ? "—" : `${Math.round(value).toLocaleString()} ips`;

export const DiagnosticsPanel: Component<DiagnosticsPanelProps> = (props) => {
  const [activeTab, setActiveTab] = createSignal<DiagnosticsTab>(props.runtimeError ? "errors" : "performance");
  const visibleSamples = createMemo(() => props.performanceSamples.slice(-48));
  const recentRows = createMemo(() => [...props.performanceSamples].slice(-6).reverse());
  const maxIps = createMemo(() => Math.max(1, ...visibleSamples().map((sample) => sample.instructionsPerSecond ?? 0)));

  createEffect(() => {
    if (props.runtimeError?.id != null) {
      setActiveTab("errors");
    }
  });

  return (
    <Panel
      title="Diagnostics"
      grow
      actions={(
        <div class="diagnostics__tabs" role="tablist" aria-label="Diagnostics views">
          <button
            type="button"
            role="tab"
            aria-selected={activeTab() === "performance"}
            class={`diagnostics__tab${activeTab() === "performance" ? " diagnostics__tab--active" : ""}`}
            onClick={() => setActiveTab("performance")}
          >
            Performance
          </button>
          <button
            type="button"
            role="tab"
            aria-selected={activeTab() === "errors"}
            class={`diagnostics__tab${activeTab() === "errors" ? " diagnostics__tab--active" : ""}`}
            onClick={() => setActiveTab("errors")}
          >
            Errors
          </button>
          <Show when={activeTab() === "errors" && props.runtimeError && props.onClearError}>
            <button type="button" class="diagnostics__clear" onClick={() => props.onClearError?.()}>
              Clear
            </button>
          </Show>
        </div>
      )}
    >
      <Show
        when={activeTab() === "performance"}
        fallback={(
          <Show
            when={props.runtimeError}
            fallback={<div class="diag-error__empty">No runtime errors recorded for this session.</div>}
          >
            {(report) => (
              <div class="diag-error">
                <div class="diag-error__summary">
                  <div>
                    <div class="diag-error__title">{report().title}</div>
                    <div class="diag-error__message">{report().summary}</div>
                  </div>
                  <button
                    type="button"
                    class="diag-error__jump"
                    onClick={() => props.onJump?.(report().pc >>> 0)}
                  >
                    Jump to faulting instruction
                  </button>
                </div>

                <div class="diag-error__meta">
                  <span class="diag-error__meta-label">Source</span>
                  <span>{report().source}</span>
                  <span class="diag-error__meta-label">PC</span>
                  <span>{hex32(report().pc)}</span>
                  <span class="diag-error__meta-label">Steps</span>
                  <span>{report().stepCount.toLocaleString()}</span>
                  <Show when={report().affectedAddress != null}>
                    <span class="diag-error__meta-label">Address</span>
                    <span>{hex32(report().affectedAddress ?? 0)}</span>
                  </Show>
                </div>

                <Show when={report().faultingInstruction}>
                  {(instruction) => (
                    <div class="diag-error__section">
                      <div class="diag-error__section-title">Faulting instruction</div>
                      <button
                        type="button"
                        class="diag-error__trace-row"
                        onClick={() => props.onJump?.(report().pc >>> 0)}
                      >
                        <span>{instruction().step != null ? instruction().step : "—"}</span>
                        <span>{hex32(report().pc)}</span>
                        <span>{hex32(instruction().raw)}</span>
                        <span class="diag-error__trace-mnem">{instruction().mnemonic}</span>
                        <span class="diag-error__trace-operands">{instruction().operands || "—"}</span>
                      </button>
                    </div>
                  )}
                </Show>

                <Show when={report().assembledSourceLocation}>
                  {(location) => (
                    <div class="diag-error__section">
                      <div class="diag-error__section-title">Assembled source</div>
                      <div class="diag-error__source-card">
                        <div class="diag-error__source-meta">
                          <span>Line {location().line}</span>
                          <span>{hex32(location().startAddr)}-{hex32((location().endAddr - 1) >>> 0)}</span>
                        </div>
                        <code class="diag-error__source-text">{location().text || "(blank source line)"}</code>
                      </div>
                    </div>
                  )}
                </Show>

                <div class="diag-error__section">
                  <div class="diag-error__section-title">Previous {props.errorContextSteps ?? report().previousSteps.length} steps</div>
                  <Show
                    when={report().previousSteps.length > 0}
                    fallback={<div class="diag-error__empty">No earlier trace entries were captured before this halt.</div>}
                  >
                    <div class="diag-error__trace-head">
                      <span>step</span>
                      <span>pc</span>
                      <span>raw</span>
                      <span>mnem</span>
                      <span>operands</span>
                    </div>
                    <For each={report().previousSteps}>
                      {(entry) => (
                        <button
                          type="button"
                          class="diag-error__trace-row"
                          onClick={() => props.onJump?.(entry.pc >>> 0)}
                        >
                          <span>{entry.step}</span>
                          <span>{hex32(entry.pc)}</span>
                          <span>{hex32(entry.raw)}</span>
                          <span class="diag-error__trace-mnem">{entry.mnemonic}</span>
                          <span class="diag-error__trace-operands">{entry.operands || "—"}</span>
                        </button>
                      )}
                    </For>
                  </Show>
                </div>
              </div>
            )}
          </Show>
        )}
      >
        <Show
          when={props.performanceSamples.length > 0}
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
      </Show>
    </Panel>
  );
};