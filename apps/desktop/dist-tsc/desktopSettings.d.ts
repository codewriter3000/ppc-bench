export interface DesktopSettings {
    dolphin_path: string | null;
    dolphin_enable_mmu: boolean;
    dark_theme: boolean;
    disassembly_line_limit: number;
    error_context_steps: number;
}
export declare const DEFAULT_DESKTOP_SETTINGS: DesktopSettings;
export declare const applyDesktopTheme: (settings: DesktopSettings) => void;
export declare function loadDesktopSettings(): Promise<DesktopSettings>;
export declare function saveDesktopSettings(settings: DesktopSettings): Promise<DesktopSettings>;
export declare function pickDolphinPath(): Promise<string | null>;
export declare function listenForDesktopSettingsUpdates(onUpdate: (settings: DesktopSettings) => void): Promise<() => void>;
