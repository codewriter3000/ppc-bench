import { createSignal, onCleanup, onMount } from "solid-js";
import { getCurrentWindow } from "@tauri-apps/api/window";
/** Returns the Tauri window handle, or null when running in a plain browser. */
// Tauri v2 injects `__TAURI_INTERNALS__` (v1 used `__TAURI_IPC__`).
const hasTauri = typeof window !== "undefined" &&
    ("__TAURI_INTERNALS__" in window || "__TAURI_IPC__" in window);
const tauriWindow = () => (hasTauri ? getCurrentWindow() : null);
export function DesktopTopbar(props) {
    const [isMaximized, setIsMaximized] = createSignal(false);
    const devWatermark = import.meta.env.VITE_DEV_WATERMARK ?? "";
    let removeResizeListener;
    const sync = async () => {
        const win = tauriWindow();
        if (!win)
            return;
        try {
            setIsMaximized(await win.isMaximized());
        }
        catch { /* ignore */ }
    };
    const minimize = async () => {
        const win = tauriWindow();
        if (!win)
            return;
        try {
            await win.minimize();
        }
        catch { /* ignore */ }
    };
    const toggleMaximize = async () => {
        const win = tauriWindow();
        if (!win)
            return;
        try {
            if (await win.isMaximized()) {
                await win.unmaximize();
                setIsMaximized(false);
            }
            else {
                await win.maximize();
                setIsMaximized(true);
            }
        }
        catch { /* ignore */ }
    };
    const close = async () => {
        const win = tauriWindow();
        if (!win)
            return;
        try {
            await win.close();
        }
        catch { /* ignore */ }
    };
    onMount(() => {
        const win = tauriWindow();
        if (!win)
            return;
        void sync();
        void win.onResized(() => void sync()).then((u) => { removeResizeListener = u; });
    });
    onCleanup(() => removeResizeListener?.());
    return (<header class="desktop-topbar">
      <style>{`
        .desktop-topbar {
          height: var(--topbar-h, 48px);
          display: flex;
          align-items: center;
          justify-content: space-between;
          gap: 16px;
          padding: 0 0 0 18px;
          border-bottom: 1px solid var(--color-border-soft, #d9dde6);
          background: var(--color-topbar-bg, rgba(255,255,255,0.92));
          backdrop-filter: blur(14px);
          color: var(--color-text, #172033);
          user-select: none;
          flex-shrink: 0;
        }
        .desktop-topbar__drag-region {
          min-width: 0;
          flex: 1;
          height: 100%;
          display: flex;
          align-items: center;
          gap: 10px;
        }
        .desktop-topbar__title {
          font-size: 0.95rem;
          font-weight: 600;
          white-space: nowrap;
        }
        .desktop-topbar__subtitle {
          font-size: 0.78rem;
          color: var(--color-text-muted, #52607a);
          white-space: nowrap;
          overflow: hidden;
          text-overflow: ellipsis;
          min-width: 0;
        }
        .desktop-topbar__watermark {
          font-size: 0.7rem;
          font-weight: 700;
          text-transform: uppercase;
          letter-spacing: 0.5px;
          color: #fff;
          background: linear-gradient(135deg, #ff6b6b 0%, #ee5a6f 100%);
          padding: 2px 8px;
          border-radius: 4px;
          margin-left: auto;
        }
        .desktop-topbar__controls {
          display: flex;
          align-items: center;
          gap: 2px;
          padding-right: 4px;
        }
        .desktop-topbar__btn {
          width: 34px;
          height: 34px;
          display: inline-flex;
          align-items: center;
          justify-content: center;
          border: none;
          border-radius: 8px;
          background: transparent;
          color: var(--color-text-muted, #52607a);
          cursor: pointer;
          font: inherit;
          font-size: 0.9rem;
          font-weight: 700;
          line-height: 1;
          transition: background-color 120ms ease, color 120ms ease;
        }
        .desktop-topbar__btn:hover {
          background: var(--color-topbar-hover, #e6ebf3);
          color: var(--color-text, #172033);
        }
        .desktop-topbar__btn:focus-visible {
          outline: 2px solid var(--color-primary, #005fcc);
          outline-offset: 2px;
        }
        .desktop-topbar__btn--close:hover {
          background: var(--color-topbar-close-hover, rgba(209,67,67,0.13));
          color: var(--color-topbar-close-text, #9d2a2a);
        }
        .desktop-topbar__glyph--minimize { display:block; transform: translateY(-3px); }
        .desktop-topbar__glyph--maximize {
          display: inline-block;
          font-size: 1.8em;
          transform: translateY(-4px);
          -webkit-text-stroke: 1px var(--color-text-muted, #52607a);
        }
        .desktop-topbar__btn:hover .desktop-topbar__glyph--maximize {
          -webkit-text-stroke: 1px var(--color-text, #172033);
        }
        .desktop-topbar__glyph--close {
          display:block;
          transform: translateY(-2px);
          -webkit-text-stroke: 1px var(--color-text-muted, #52607a);
        }
        .desktop-topbar__btn--close:hover .desktop-topbar__glyph--close {
          -webkit-text-stroke: 1px var(--color-topbar-close-text, #9d2a2a);
        }
      `}</style>
  {devWatermark && (<span class="desktop-topbar__watermark">{devWatermark}</span>)}

      {/* Drag region — double-click to toggle maximize */}
      <div class="desktop-topbar__drag-region" data-tauri-drag-region onDblClick={() => void toggleMaximize()}>
        <span class="desktop-topbar__title">{props.title ?? "PPC-Bench"}</span>
        {props.subtitle && (<span class="desktop-topbar__subtitle">{props.subtitle}</span>)}
      </div>

      {/* Window controls */}
      <div class="desktop-topbar__controls" role="group" aria-label="Window controls">
        {props.onSettings && (<button type="button" class="desktop-topbar__btn" aria-label="Settings" title="Settings" onClick={props.onSettings}>
            <span aria-hidden="true">⚙</span>
          </button>)}
        <button type="button" class="desktop-topbar__btn" aria-label="Minimize" title="Minimize" onClick={() => void minimize()}>
          <span class="desktop-topbar__glyph--minimize" aria-hidden="true">—</span>
        </button>
        <button type="button" class="desktop-topbar__btn" aria-label={isMaximized() ? "Restore" : "Maximize"} title={isMaximized() ? "Restore" : "Maximize"} onClick={() => void toggleMaximize()}>
          <span class="desktop-topbar__glyph--maximize" aria-hidden="true">
            {isMaximized() ? "❐" : "□"}
          </span>
        </button>
        <button type="button" class="desktop-topbar__btn desktop-topbar__btn--close" aria-label="Close" title="Close" onClick={() => void close()}>
          <span class="desktop-topbar__glyph--close" aria-hidden="true">✕</span>
        </button>
      </div>
    </header>);
}
