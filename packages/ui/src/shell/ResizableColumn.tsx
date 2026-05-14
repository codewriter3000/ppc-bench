/**
 * ResizableColumn
 *
 * Renders a vertical stack of panels with `.split-handle--row` drag bars
 * placed *between* adjacent panels — not embedded inside any panel border.
 *
 * Items without `initialHeight` grow to fill remaining space.
 * Items with `initialHeight` get a fixed height that the user can drag.
 * The handle is always the element between two panels in the DOM.
 */
import { type Component, createSignal, JSX } from "solid-js";
import "../styles/shell.css";

export interface ColumnItem {
  /** The panel JSX node to render. */
  node: JSX.Element;
  /**
   * Fixed starting height in px.  When omitted the item grows (`flex: 1 1 0`)
   * and no drag handle is placed after it.
   */
  initialHeight?: number;
  /** Minimum height when dragging. Defaults to 80. */
  minHeight?: number;
}

export interface ResizableColumnProps {
  items: ColumnItem[];
}

export const ResizableColumn: Component<ResizableColumnProps> = (props) => {
  // Create one optional signal per item at initialisation time (signals cannot
  // be created conditionally or inside callbacks).
  const heights = props.items.map((item) =>
    item.initialHeight != null ? createSignal(item.initialHeight) : null,
  );

  // One "is-dragging" flag per potential handle (N-1 slots, indexed by the
  // *upper* panel's index).
  const dragging = props.items.slice(0, -1).map(() => createSignal(false));

  /**
   * Returns a pointerdown handler for the handle that sits between item `i`
   * (upper) and item `i+1` (lower).  Only items with a height signal get a
   * handle.
   */
  const makeDragHandler = (i: number) => (e: PointerEvent) => {
    const sig = heights[i];
    if (!sig) return; // grow items don't have drag handles
    e.preventDefault();

    const [getH, setH] = sig;
    const [, setD] = dragging[i]!;
    const startY = e.clientY;
    const startH = getH();
    const min = props.items[i]?.minHeight ?? 80;

    setD(true);
    const handle = e.currentTarget as HTMLElement;
    handle.setPointerCapture(e.pointerId);

    const onMove = (ev: PointerEvent) =>
      setH(Math.max(min, startH + ev.clientY - startY));

    const onUp = () => {
      setD(false);
      handle.removeEventListener("pointermove", onMove);
      handle.removeEventListener("pointerup", onUp);
    };

    handle.addEventListener("pointermove", onMove);
    handle.addEventListener("pointerup", onUp);
  };

  return (
    <div
      style="display:flex;flex-direction:column;min-height:0;flex:1 1 0;overflow:hidden;"
    >
      {props.items.map((item, i) => {
        const sig = heights[i];
        // The wrapper div controls sizing; the panel inside always fills it.
        const wrapperStyle = () =>
          sig
            ? `flex:0 0 ${sig[0]()}px;height:${sig[0]()}px;min-height:0;overflow:hidden;display:flex;flex-direction:column;`
            : "flex:1 1 0;min-height:0;overflow:hidden;display:flex;flex-direction:column;";

        // Place a split handle AFTER this item if it has a height signal
        // (i.e., there is a panel after it that it can push against).
        const showHandle = sig != null && i < props.items.length - 1;
        const [isDragging] = dragging[i] ?? [() => false];

        return [
          <div style={wrapperStyle()}>{item.node}</div>,
          showHandle && (
            <div
              class={`split-handle split-handle--row${isDragging() ? " split-handle--active" : ""}`}
              onPointerDown={makeDragHandler(i)}
              title="Drag to resize"
            />
          ),
        ];
      })}
    </div>
  );
};
