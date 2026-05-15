import { listen } from "@tauri-apps/api/event";
import { invoke } from "@tauri-apps/api/core";

const BROWSER_SETTINGS_KEY = "ppc-bench.desktop-settings";
const BROWSER_SETTINGS_EVENT = "ppc-bench:settings-updated";
const hasTauri =
  typeof window !== "undefined" &&
  ("__TAURI_INTERNALS__" in window || "__TAURI_IPC__" in window);

export interface DesktopSettings {
  dolphin_path: string | null;
  dolphin_enable_mmu: boolean;
  dark_theme: boolean;
  disassembly_line_limit: number;
  error_context_steps: number;
}

export const DEFAULT_DESKTOP_SETTINGS: DesktopSettings = {
  dolphin_path: null,
  dolphin_enable_mmu: false,
  dark_theme: false,
  disassembly_line_limit: 1000,
  error_context_steps: 5,
};

const normalizeDesktopSettings = (settings: Partial<DesktopSettings> | null | undefined): DesktopSettings => ({
  dolphin_path: settings?.dolphin_path?.trim() ? settings.dolphin_path.trim() : null,
  dolphin_enable_mmu: settings?.dolphin_enable_mmu === true,
  dark_theme: settings?.dark_theme === true,
  disassembly_line_limit: Math.max(100, Math.min(20_000, Number(settings?.disassembly_line_limit ?? 1000) || 1000)),
  error_context_steps: Math.max(1, Math.min(50, Number(settings?.error_context_steps ?? 5) || 5)),
});

const readBrowserSettings = (): DesktopSettings => {
  try {
    const raw = window.localStorage.getItem(BROWSER_SETTINGS_KEY);
    if (!raw) {
      return DEFAULT_DESKTOP_SETTINGS;
    }

    return normalizeDesktopSettings(JSON.parse(raw) as Partial<DesktopSettings>);
  } catch {
    return DEFAULT_DESKTOP_SETTINGS;
  }
};

export const applyDesktopTheme = (settings: DesktopSettings) => {
  const theme = settings.dark_theme ? "dark" : "light";
  document.documentElement.dataset.theme = theme;
  document.body.dataset.theme = theme;
  document.documentElement.style.colorScheme = theme;
};

export async function loadDesktopSettings(): Promise<DesktopSettings> {
  if (!hasTauri) {
    return readBrowserSettings();
  }

  return normalizeDesktopSettings(await invoke<DesktopSettings>("load_settings"));
}

export async function saveDesktopSettings(settings: DesktopSettings): Promise<DesktopSettings> {
  const normalized = normalizeDesktopSettings(settings);
  if (!hasTauri) {
    window.localStorage.setItem(BROWSER_SETTINGS_KEY, JSON.stringify(normalized));
    window.dispatchEvent(new CustomEvent(BROWSER_SETTINGS_EVENT, { detail: normalized }));
    return normalized;
  }

  return normalizeDesktopSettings(await invoke<DesktopSettings>("save_settings", { settings: normalized }));
}

export async function pickDolphinPath(): Promise<string | null> {
  if (!hasTauri) {
    const value = window.prompt("Path to Dolphin.exe", "")?.trim();
    return value ? value : null;
  }

  const selected = await invoke<string | null>("pick_dolphin_path");
  return selected?.trim() ? selected.trim() : null;
}

export async function listenForDesktopSettingsUpdates(
  onUpdate: (settings: DesktopSettings) => void,
): Promise<() => void> {
  if (!hasTauri) {
    const handler = (event: Event) => {
      const detail = (event as CustomEvent<DesktopSettings>).detail;
      onUpdate(normalizeDesktopSettings(detail));
    };
    window.addEventListener(BROWSER_SETTINGS_EVENT, handler);
    return () => window.removeEventListener(BROWSER_SETTINGS_EVENT, handler);
  }

  return listen<DesktopSettings>("settings-updated", (event) => {
    onUpdate(normalizeDesktopSettings(event.payload));
  });
}