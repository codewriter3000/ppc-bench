import { createMemo, createSignal, onCleanup, onMount } from "solid-js";
import { DesktopTopbar } from "./DesktopTopbar";
import { DEFAULT_DESKTOP_SETTINGS, loadDesktopSettings, listenForDesktopSettingsUpdates, pickDolphinPath, saveDesktopSettings, } from "./desktopSettings";
import "./settings.css";
export const SettingsApp = () => {
    const [settings, setSettings] = createSignal(DEFAULT_DESKTOP_SETTINGS);
    const [savedSettings, setSavedSettings] = createSignal(DEFAULT_DESKTOP_SETTINGS);
    const [loading, setLoading] = createSignal(true);
    const [saving, setSaving] = createSignal(false);
    const [status, setStatus] = createSignal(null);
    let removeSettingsListener;
    const dirty = createMemo(() => {
        const current = settings();
        const saved = savedSettings();
        return current.dolphin_path !== saved.dolphin_path
            || current.dolphin_enable_mmu !== saved.dolphin_enable_mmu
            || current.dark_theme !== saved.dark_theme
            || current.disassembly_line_limit !== saved.disassembly_line_limit
            || current.error_context_steps !== saved.error_context_steps;
    });
    onMount(() => {
        document.title = "PPC Settings";
        void loadDesktopSettings()
            .then((loaded) => {
            setSettings(loaded);
            setSavedSettings(loaded);
        })
            .catch((err) => {
            setStatus({ tone: "error", message: String(err) });
        })
            .finally(() => {
            setLoading(false);
        });
        void listenForDesktopSettingsUpdates((next) => {
            setSavedSettings(next);
            setSettings((current) => (dirty() ? current : next));
        }).then((dispose) => {
            removeSettingsListener = dispose;
        });
    });
    onCleanup(() => removeSettingsListener?.());
    const updateSettings = (patch) => {
        setSettings((current) => ({ ...current, ...patch }));
        setStatus(null);
    };
    const onBrowse = async () => {
        try {
            const selected = await pickDolphinPath();
            if (selected) {
                updateSettings({ dolphin_path: selected });
            }
        }
        catch (err) {
            setStatus({ tone: "error", message: String(err) });
        }
    };
    const onSave = async () => {
        setSaving(true);
        setStatus(null);
        try {
            const saved = await saveDesktopSettings(settings());
            setSettings(saved);
            setSavedSettings(saved);
            setStatus({ tone: "success", message: "Settings saved." });
        }
        catch (err) {
            setStatus({ tone: "error", message: String(err) });
        }
        finally {
            setSaving(false);
        }
    };
    return (<div class="settings-app">
      <DesktopTopbar title="PPC Settings"/>
      <main class="settings-shell">
        <section class="settings-card" aria-busy={loading() ? "true" : "false"}>
          <header class="settings-card__header">
            <div>
              <h1 class="settings-card__title">Desktop Settings</h1>
              <p class="settings-card__subtitle">Saved in the app config directory and restored on next launch.</p>
            </div>
          </header>

          <div class="settings-form">
            <label class="settings-field">
              <span class="settings-field__label">Dolphin executable path</span>
              <span class="settings-field__hint">Leave blank to keep using auto-detect and the fallback picker.</span>
              <div class="settings-path-row">
                <input class="settings-input" type="text" value={settings().dolphin_path ?? ""} onInput={(event) => updateSettings({ dolphin_path: event.currentTarget.value || null })} placeholder="C:\\Program Files\\Dolphin\\Dolphin.exe" spellcheck={false}/>
                <button type="button" class="btn" onClick={() => void onBrowse()}>
                  Browse
                </button>
                <button type="button" class="btn" onClick={() => updateSettings({ dolphin_path: null })}>
                  Clear
                </button>
              </div>
            </label>

            <label class="settings-toggle">
              <span>
                <span class="settings-field__label">Enable Dolphin MMU</span>
                <span class="settings-field__hint">Adds `Main.Core.MMU=true` to the Dolphin launch command. Enable this for binaries that fault or warn without MMU.</span>
              </span>
              <input class="settings-toggle__input" type="checkbox" checked={settings().dolphin_enable_mmu} onChange={(event) => updateSettings({ dolphin_enable_mmu: event.currentTarget.checked })}/>
            </label>

            <label class="settings-toggle">
              <span>
                <span class="settings-field__label">Disassembly line limit</span>
                <span class="settings-field__hint">Only this many disassembly rows are rendered at once. Lower values keep large DOL and ELF listings responsive.</span>
              </span>
              <input class="settings-input settings-input--short" type="number" min="100" max="20000" step="100" value={String(settings().disassembly_line_limit)} onInput={(event) => updateSettings({ disassembly_line_limit: Number.parseInt(event.currentTarget.value, 10) || DEFAULT_DESKTOP_SETTINGS.disassembly_line_limit })}/>
            </label>

            <label class="settings-toggle">
              <span>
                <span class="settings-field__label">Error history depth</span>
                <span class="settings-field__hint">How many previous executed steps are included in the Errors tab when execution halts unexpectedly.</span>
              </span>
              <input class="settings-input settings-input--short" type="number" min="1" max="50" step="1" value={String(settings().error_context_steps)} onInput={(event) => updateSettings({ error_context_steps: Number.parseInt(event.currentTarget.value, 10) || DEFAULT_DESKTOP_SETTINGS.error_context_steps })}/>
            </label>

            <label class="settings-toggle">
              <span>
                <span class="settings-field__label">Dark theme</span>
                <span class="settings-field__hint">Applies to the main window, manuals, and this settings window.</span>
              </span>
              <input class="settings-toggle__input" type="checkbox" checked={settings().dark_theme} onChange={(event) => updateSettings({ dark_theme: event.currentTarget.checked })}/>
            </label>
          </div>

          <footer class="settings-footer">
            <span class={`settings-status${status() ? ` settings-status--${status().tone}` : ""}`}>
              {status()?.message ?? (dirty() ? "Unsaved changes." : "No pending changes.")}
            </span>
            <button type="button" class="btn btn--primary" disabled={loading() || saving() || !dirty()} onClick={() => void onSave()}>
              {saving() ? "Saving…" : "Save Settings"}
            </button>
          </footer>
        </section>
      </main>
    </div>);
};
