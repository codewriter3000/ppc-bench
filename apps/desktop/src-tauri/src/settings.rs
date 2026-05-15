use std::fs;
use std::path::Path;

use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter, Manager};

use crate::dolphin::pick_dolphin_executable;

const SETTINGS_FILE: &str = "settings.json";
pub const SETTINGS_UPDATED_EVENT: &str = "settings-updated";

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct AppSettings {
    pub dolphin_path: Option<String>,
    pub dolphin_enable_mmu: bool,
    pub dark_theme: bool,
    pub disassembly_line_limit: u32,
    pub error_context_steps: u32,
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            dolphin_path: None,
            dolphin_enable_mmu: false,
            dark_theme: false,
            disassembly_line_limit: 1000,
            error_context_steps: 5,
        }
    }
}

#[tauri::command]
pub fn load_settings(app: AppHandle) -> Result<AppSettings, String> {
    load_app_settings(&app)
}

#[tauri::command]
pub fn save_settings(app: AppHandle, settings: AppSettings) -> Result<AppSettings, String> {
    let normalized = normalize_settings(settings)?;
    let path = settings_path(&app)?;
    let parent = path
        .parent()
        .ok_or_else(|| format!("settings path has no parent: {}", path.display()))?;
    fs::create_dir_all(parent)
        .map_err(|err| format!("failed to create {}: {err}", parent.display()))?;

    let json = serde_json::to_string_pretty(&normalized)
        .map_err(|err| format!("failed to serialize settings: {err}"))?;
    fs::write(&path, json)
        .map_err(|err| format!("failed to write {}: {err}", path.display()))?;

    app.emit(SETTINGS_UPDATED_EVENT, &normalized)
        .map_err(|err| format!("failed to emit settings update: {err}"))?;

    Ok(normalized)
}

#[tauri::command]
pub fn pick_dolphin_path(app: AppHandle) -> Result<Option<String>, String> {
    let selected = pick_dolphin_executable(&app)?;
    Ok(selected.map(|path| path.display().to_string()))
}

pub fn load_app_settings(app: &AppHandle) -> Result<AppSettings, String> {
    let path = settings_path(app)?;
    if !path.is_file() {
        return Ok(AppSettings::default());
    }

    let json = fs::read_to_string(&path)
        .map_err(|err| format!("failed to read {}: {err}", path.display()))?;
    let settings = serde_json::from_str::<AppSettings>(&json)
        .map_err(|err| format!("failed to parse {}: {err}", path.display()))?;
    normalize_settings(settings)
}

fn settings_path(app: &AppHandle) -> Result<std::path::PathBuf, String> {
    let dir = app
        .path()
        .app_config_dir()
        .map_err(|err| format!("failed to resolve app config directory: {err}"))?;
    Ok(dir.join(SETTINGS_FILE))
}

fn normalize_settings(settings: AppSettings) -> Result<AppSettings, String> {
    let dolphin_path = settings
        .dolphin_path
        .and_then(|path| {
            let trimmed = path.trim().to_string();
            if trimmed.is_empty() {
                None
            } else {
                Some(trimmed)
            }
        });

    if let Some(path) = &dolphin_path {
        let path_obj = Path::new(path);
        if !path_obj.is_file() {
            return Err(format!("Dolphin executable does not exist: {path}"));
        }
    }

    Ok(AppSettings {
        dolphin_path,
        dolphin_enable_mmu: settings.dolphin_enable_mmu,
        dark_theme: settings.dark_theme,
        disassembly_line_limit: settings.disassembly_line_limit.clamp(100, 20_000),
        error_context_steps: settings.error_context_steps.clamp(1, 50),
    })
}