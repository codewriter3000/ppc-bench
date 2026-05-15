import { type Component, For, Show } from "solid-js";
import { hex32, hex8 } from "../styles/format";
import { Panel } from "../shell/Panel";
import "../styles/panels-misc.css";

export interface SnapshotDiffEntry {
  id: string;
  addr: number;
  size: number;
  label?: string;
  beforeBytes: readonly number[];
  afterBytes: readonly number[];
}

export interface SnapshotDiffPanelProps {
  entries: readonly SnapshotDiffEntry[];
  comparisonLabel?: string;
  onJump?: (addr: number) => void;
}

const formatBytes = (bytes: readonly number[]) => bytes.map((byte) => hex8(byte)).join(" ");

export const SnapshotDiffPanel: Component<SnapshotDiffPanelProps> = (props) => {
  return (
    <Panel
      title="Snapshot Diff"
      grow
      actions={<span>{props.entries.length}{props.comparisonLabel ? ` • ${props.comparisonLabel}` : ""}</span>}
    >
      <Show
        when={props.entries.length > 0}
        fallback={<div class="snapshot-diff__empty">Pause Dolphin on at least two snapshots to inspect changed memory regions.</div>}
      >
        <For each={props.entries}>
          {(entry) => (
            <div
              class="snapshot-diff__row"
              onClick={() => props.onJump?.(entry.addr >>> 0)}
              title="Jump to this address in Memory"
            >
              <div class="snapshot-diff__meta">
                <span class="snapshot-diff__addr">{hex32(entry.addr)}</span>
                <span class="snapshot-diff__size">{entry.size} byte{entry.size === 1 ? "" : "s"}</span>
                <span class="snapshot-diff__label">{entry.label ?? "changed bytes"}</span>
              </div>
              <div class="snapshot-diff__bytes">
                <span class="snapshot-diff__before">{formatBytes(entry.beforeBytes)}</span>
                <span class="snapshot-diff__arrow">→</span>
                <span class="snapshot-diff__after">{formatBytes(entry.afterBytes)}</span>
              </div>
            </div>
          )}
        </For>
      </Show>
    </Panel>
  );
};