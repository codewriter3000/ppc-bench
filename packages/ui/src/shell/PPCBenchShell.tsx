import { JSX, type Component, createSignal } from "solid-js";
import "../styles/tokens.css";
import "../styles/shell.css";

export interface PPCBenchShellProps {
  /** Optional custom title bar / window chrome row. */
  titleBar?: JSX.Element;
  topBar?: JSX.Element;
  left?: JSX.Element;
  center?: JSX.Element;
  right?: JSX.Element;
  bottom?: JSX.Element;
  initialLeftWidth?: number;
  initialRightWidth?: number;
  initialBottomHeight?: number;
}

const MIN_COL = 180;
const MIN_BOTTOM = 60;

/** Creates a pointer-capture drag handler for a split handle element. */
function useSplitter(
  getValue: () => number,
  setValue: (v: number) => void,
  axis: "x" | "y",
  sign: 1 | -1 = 1,
  min = MIN_COL,
) {
  return (e: PointerEvent) => {
    e.preventDefault();
    const start = axis === "x" ? e.clientX : e.clientY;
    const startVal = getValue();
    const handle = e.currentTarget as HTMLElement;
    handle.setPointerCapture(e.pointerId);

    const onMove = (ev: PointerEvent) => {
      const delta = (axis === "x" ? ev.clientX : ev.clientY) - start;
      setValue(Math.max(min, startVal + sign * delta));
    };
    const onUp = () => {
      handle.releasePointerCapture(e.pointerId);
      handle.removeEventListener("pointermove", onMove);
      handle.removeEventListener("pointerup", onUp);
    };
    handle.addEventListener("pointermove", onMove);
    handle.addEventListener("pointerup", onUp);
  };
}

export const PPCBenchShell: Component<PPCBenchShellProps> = (props) => {
  const [leftW, setLeftW] = createSignal(props.initialLeftWidth ?? 460);
  const [rightW, setRightW] = createSignal(props.initialRightWidth ?? 360);
  const [bottomH, setBottomH] = createSignal(props.initialBottomHeight ?? 200);
  const [draggingLeft, setDraggingLeft] = createSignal(false);
  const [draggingRight, setDraggingRight] = createSignal(false);
  const [draggingBottom, setDraggingBottom] = createSignal(false);

  const makeDragger = (
    get: () => number,
    set: (v: number) => void,
    setActive: (b: boolean) => void,
    axis: "x" | "y",
    sign: 1 | -1 = 1,
    min?: number,
  ) => (e: PointerEvent) => {
    e.preventDefault();
    setActive(true);
    const start = axis === "x" ? e.clientX : e.clientY;
    const startVal = get();
    const handle = e.currentTarget as HTMLElement;
    handle.setPointerCapture(e.pointerId);
    const onMove = (ev: PointerEvent) => {
      const delta = (axis === "x" ? ev.clientX : ev.clientY) - start;
      set(Math.max(min ?? MIN_COL, startVal + sign * delta));
    };
    const onUp = () => {
      setActive(false);
      handle.releasePointerCapture(e.pointerId);
      handle.removeEventListener("pointermove", onMove);
      handle.removeEventListener("pointerup", onUp);
    };
    handle.addEventListener("pointermove", onMove);
    handle.addEventListener("pointerup", onUp);
  };

  const onDragLeft = makeDragger(leftW, setLeftW, setDraggingLeft, "x");
  const onDragRight = makeDragger(rightW, setRightW, setDraggingRight, "x", -1);
  const onDragBottom = makeDragger(bottomH, setBottomH, setDraggingBottom, "y", -1, MIN_BOTTOM);

  return (
    <div class="shell">
      {props.titleBar}
      {props.topBar}

      <div class="shell__body">
        {/* Left column */}
        <div class="shell__column" style={`flex:0 0 ${leftW()}px;width:${leftW()}px;`}>
          {props.left}
        </div>

        {/* Left ↔ Center splitter */}
        <div
          class={`split-handle split-handle--col${draggingLeft() ? " split-handle--active" : ""}`}
          onPointerDown={onDragLeft}
          title="Drag to resize"
        />

        {/* Center column — fills remaining space */}
        <div class="shell__column shell__column--center">
          {props.center}
        </div>

        {/* Center ↔ Right splitter */}
        <div
          class={`split-handle split-handle--col${draggingRight() ? " split-handle--active" : ""}`}
          onPointerDown={onDragRight}
          title="Drag to resize"
        />

        {/* Right column */}
        <div class="shell__column" style={`flex:0 0 ${rightW()}px;width:${rightW()}px;`}>
          {props.right}
        </div>
      </div>

      {/* Body / Bottom splitter */}
      {props.bottom && (
        <div
          class={`split-handle split-handle--row${draggingBottom() ? " split-handle--active" : ""}`}
          onPointerDown={onDragBottom}
          title="Drag to resize"
        />
      )}

      {props.bottom && (
        <div class="shell__bottom" style={`height:${bottomH()}px;`}>
          {props.bottom}
        </div>
      )}
    </div>
  );
};
