import { WebviewWindow } from "@tauri-apps/api/webviewWindow";
const hasTauri = typeof window !== "undefined" &&
    ("__TAURI_INTERNALS__" in window || "__TAURI_IPC__" in window);
const MANUALS_LABEL = "manuals";
/**
 * Opens (or focuses) the PPC reference manuals window. Falls back to a plain
 * browser tab when running outside Tauri (e.g. `vite dev` in a browser).
 */
export async function openManualsWindow() {
    if (!hasTauri) {
        window.open("/#manuals", "_blank");
        return;
    }
    try {
        const existing = await WebviewWindow.getByLabel(MANUALS_LABEL);
        if (existing) {
            await existing.show();
            await existing.setFocus();
            return;
        }
    }
    catch {
        /* fall through to create */
    }
    const win = new WebviewWindow(MANUALS_LABEL, {
        url: "index.html#manuals",
        title: "PPC Manuals",
        width: 1200,
        height: 800,
        minWidth: 720,
        minHeight: 500,
        resizable: true,
        decorations: false,
    });
    win.once("tauri://error", (e) => {
        console.error("Failed to open manuals window", e);
    });
}
