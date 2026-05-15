import { WebviewWindow } from "@tauri-apps/api/webviewWindow";

const hasTauri =
  typeof window !== "undefined" &&
  ("__TAURI_INTERNALS__" in window || "__TAURI_IPC__" in window);

const SETTINGS_LABEL = "settings";

export async function openSettingsWindow(): Promise<void> {
  if (!hasTauri) {
    window.open("/#settings", "_blank");
    return;
  }

  try {
    const existing = await WebviewWindow.getByLabel(SETTINGS_LABEL);
    if (existing) {
      await existing.show();
      await existing.setFocus();
      return;
    }
  } catch {
    /* fall through to create */
  }

  const win = new WebviewWindow(SETTINGS_LABEL, {
    url: "index.html#settings",
    title: "PPC Settings",
    width: 620,
    height: 560,
    minWidth: 420,
    minHeight: 440,
    resizable: true,
    decorations: false,
  });

  win.once("tauri://error", (event) => {
    console.error("Failed to open settings window", event);
  });
}