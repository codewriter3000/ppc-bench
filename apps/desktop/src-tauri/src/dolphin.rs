//! Dolphin process discovery and launch helpers.

use std::env;
use std::fs::{self, File};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};

use tauri::AppHandle;
use tauri_plugin_dialog::DialogExt;

pub const DEFAULT_GDB_PORT: u16 = 55_000;
pub const DOLPHIN_BINARY: &str = "Dolphin.exe";
const USER_DIR_NAME: &str = "ppc-bench-dolphin-user";
const DOLPHIN_USER_DIR_ENV: &str = "DOLPHIN_USER_DIR";
const STDOUT_LOG_NAME: &str = "ppc-bench-dolphin-stdout.log";
const STDERR_LOG_NAME: &str = "ppc-bench-dolphin-stderr.log";
const MAX_LOG_EXCERPT_CHARS: usize = 320;

pub struct DolphinSession {
    child: Child,
    stdout_log_path: PathBuf,
    stderr_log_path: PathBuf,
}

pub fn default_gdb_port() -> u16 {
    DEFAULT_GDB_PORT
}

pub fn find_dolphin() -> Option<PathBuf> {
    if let Some(path) = env::var_os("DOLPHIN_PATH").map(PathBuf::from) {
        if path.is_file() {
            return Some(path);
        }
    }

    for candidate in candidate_paths() {
        if candidate.is_file() {
            return Some(candidate);
        }
    }

    None
}

pub fn pick_dolphin_executable(app: &AppHandle) -> Result<Option<PathBuf>, String> {
    let selected = app
        .dialog()
        .file()
        .add_filter("Executable", &["exe"])
        .set_file_name(DOLPHIN_BINARY)
        .blocking_pick_file();

    let Some(selected) = selected else {
        return Ok(None);
    };

    let path = selected
        .into_path()
        .map_err(|_| "selected Dolphin path is not available on the local filesystem".to_string())?;

    if !path.is_file() {
        return Err(format!("selected Dolphin path is not a file: {}", path.display()));
    }

    Ok(Some(path))
}

pub fn find_dolphin_with_picker(app: &AppHandle, preferred_path: Option<&str>) -> Result<PathBuf, String> {
    if let Some(path) = preferred_path.map(PathBuf::from) {
        if path.is_file() {
            return Ok(path);
        }
    }

    if let Some(path) = find_dolphin() {
        return Ok(path);
    }

    pick_dolphin_executable(app)?
        .ok_or_else(|| {
            "Dolphin executable not found automatically and no file was selected.".to_string()
        })
}

pub fn write_temp_dol(dol_bytes: &[u8]) -> Result<PathBuf, String> {
    write_temp_launch_image(dol_bytes, "dol")
}

pub fn write_temp_launch_image(bytes: &[u8], extension: &str) -> Result<PathBuf, String> {
    let normalized_extension = extension.trim().trim_start_matches('.');
    let file_name = if normalized_extension.is_empty() {
        "ppc-bench-run.bin".to_string()
    } else {
        format!("ppc-bench-run.{normalized_extension}")
    };
    let path = env::temp_dir().join(file_name);
    fs::write(&path, bytes)
        .map_err(|err| format!("failed to write {}: {err}", path.display()))?;
    Ok(path)
}

fn build_dolphin_command(
    dolphin_path: &Path,
    dol_path: &Path,
    gdb_port: u16,
    user_dir: &Path,
    enable_mmu: bool,
) -> Command {
    let mut command = Command::new(dolphin_path);
    command
        .arg("--user")
        .arg(user_dir)
        .arg("--batch")
        .arg("--debugger")
        .arg("--config")
        .arg("Main.Analytics.PermissionAsked=true")
        .arg("--config")
        .arg("Main.Analytics.Enabled=false")
        .arg("--config")
        .arg(format!("Main.Core.MMU={}", if enable_mmu { "true" } else { "false" }))
        .arg("--config")
        .arg("Main.General.GDBSocket=")
        .arg("--config")
        .arg(format!("Main.General.GDBPort={gdb_port}"))
        .arg("--exec")
        .arg(dol_path);
    command
}

pub fn launch_dolphin(
    dolphin_path: &Path,
    dol_path: &Path,
    gdb_port: u16,
    enable_mmu: bool,
) -> Result<DolphinSession, String> {
    let user_dir = prepare_user_dir()?;
    let (stdout_file, stdout_log_path) = create_log_file(STDOUT_LOG_NAME)?;
    let (stderr_file, stderr_log_path) = create_log_file(STDERR_LOG_NAME)?;

    let mut command = build_dolphin_command(dolphin_path, dol_path, gdb_port, &user_dir, enable_mmu);
    let child = command
        .stdout(Stdio::from(stdout_file))
        .stderr(Stdio::from(stderr_file))
        .spawn()
        .map_err(|err| format!("failed to launch {}: {err}", dolphin_path.display()))?;

    Ok(DolphinSession {
        child,
        stdout_log_path,
        stderr_log_path,
    })
}

pub fn describe_dolphin_exit(session: &mut DolphinSession, context: &str) -> Result<Option<String>, String> {
    let Some(status) = session
        .child
        .try_wait()
        .map_err(|err| format!("failed to query Dolphin status: {err}"))?
    else {
        return Ok(None);
    };

    let mut message = format!("{context} Dolphin exited with {}.", format_exit_status(status.code()));
    let captured_output = summarize_captured_output(session)?;
    if !captured_output.is_empty() {
        message.push(' ');
        message.push_str(&captured_output);
    }

    Ok(Some(message))
}

pub fn stop_session(session: &mut DolphinSession) -> Result<(), String> {
    stop_child(&mut session.child)
}

pub fn stop_child(child: &mut Child) -> Result<(), String> {
    if child
        .try_wait()
        .map_err(|err| format!("failed to query Dolphin status: {err}"))?
        .is_some()
    {
        return Ok(());
    }

    child.kill().map_err(|err| format!("failed to stop Dolphin: {err}"))?;
    let _ = child.wait();
    Ok(())
}

fn candidate_paths() -> Vec<PathBuf> {
    let mut candidates = Vec::new();

    if let Some(local_app_data) = env::var_os("LOCALAPPDATA") {
        candidates.push(
            PathBuf::from(local_app_data)
                .join("Programs")
                .join("Dolphin")
                .join(DOLPHIN_BINARY),
        );
    }

    for env_name in ["ProgramFiles", "ProgramFiles(x86)"] {
        if let Some(program_files) = env::var_os(env_name) {
            candidates.push(PathBuf::from(program_files).join("Dolphin").join(DOLPHIN_BINARY));
        }
    }

    candidates
}

fn preferred_user_dir() -> Option<PathBuf> {
    if let Some(path) = env::var_os(DOLPHIN_USER_DIR_ENV).map(PathBuf::from) {
        if path.is_dir() {
            return Some(path);
        }
    }

    let app_data = env::var_os("APPDATA").map(PathBuf::from)?;
    let path = app_data.join("Dolphin Emulator");
    path.is_dir().then_some(path)
}

fn prepare_user_dir() -> Result<PathBuf, String> {
    if let Some(path) = preferred_user_dir() {
        return Ok(path);
    }

    let path = env::temp_dir().join(USER_DIR_NAME);
    fs::create_dir_all(&path)
        .map_err(|err| format!("failed to create Dolphin user dir {}: {err}", path.display()))?;
    Ok(path)
}

fn create_log_file(name: &str) -> Result<(File, PathBuf), String> {
    let path = env::temp_dir().join(name);
    let file = File::create(&path)
        .map_err(|err| format!("failed to create Dolphin log {}: {err}", path.display()))?;
    Ok((file, path))
}

fn summarize_captured_output(session: &DolphinSession) -> Result<String, String> {
    let stderr_excerpt = read_log_excerpt(&session.stderr_log_path)?;
    let stdout_excerpt = read_log_excerpt(&session.stdout_log_path)?;

    if let Some(stderr) = stderr_excerpt {
        return Ok(format!("stderr: {stderr}"));
    }

    if let Some(stdout) = stdout_excerpt {
        return Ok(format!("stdout: {stdout}"));
    }

    Ok(format!(
        "No stdout/stderr output captured. Logs: stderr={}, stdout={}",
        session.stderr_log_path.display(),
        session.stdout_log_path.display()
    ))
}

fn read_log_excerpt(path: &Path) -> Result<Option<String>, String> {
    let bytes = match fs::read(path) {
        Ok(bytes) => bytes,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(err) => return Err(format!("failed to read Dolphin log {}: {err}", path.display())),
    };

    if bytes.is_empty() {
        return Ok(None);
    }

    let text = String::from_utf8_lossy(&bytes);
    let lines = text
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .collect::<Vec<_>>();
    if lines.is_empty() {
        return Ok(None);
    }

    let start = lines.len().saturating_sub(4);
    let excerpt = lines[start..].join(" | ");
    Ok(Some(truncate_from_end(&excerpt, MAX_LOG_EXCERPT_CHARS)))
}

fn truncate_from_end(text: &str, max_chars: usize) -> String {
    let chars = text.chars().collect::<Vec<_>>();
    if chars.len() <= max_chars {
        return text.to_string();
    }

    let keep = max_chars.saturating_sub(3);
    let tail = chars[chars.len().saturating_sub(keep)..].iter().collect::<String>();
    format!("...{tail}")
}

fn format_exit_status(code: Option<i32>) -> String {
    match code {
        Some(code) => format!("exit code {code} (0x{:08X})", code as u32),
        None => "an unknown status".to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn launch_command_uses_supported_gdb_overrides() {
        let command = build_dolphin_command(
            Path::new("Dolphin.exe"),
            Path::new("test.dol"),
            55_000,
            Path::new("user-dir"),
            false,
        );
        let args = command
            .get_args()
            .map(|arg| arg.to_string_lossy().into_owned())
            .collect::<Vec<_>>();

        assert_eq!(
            args,
            vec![
                "--user",
                "user-dir",
                "--batch",
                "--debugger",
                "--config",
                "Main.Analytics.PermissionAsked=true",
                "--config",
                "Main.Analytics.Enabled=false",
                "--config",
                "Main.Core.MMU=false",
                "--config",
                "Main.General.GDBSocket=",
                "--config",
                "Main.General.GDBPort=55000",
                "--exec",
                "test.dol",
            ]
        );
        assert!(!args.iter().any(|arg| arg == "--gdb-port"));
    }

    #[test]
    fn launch_command_can_enable_mmu() {
        let command = build_dolphin_command(
            Path::new("Dolphin.exe"),
            Path::new("test.dol"),
            55_000,
            Path::new("user-dir"),
            true,
        );
        let args = command
            .get_args()
            .map(|arg| arg.to_string_lossy().into_owned())
            .collect::<Vec<_>>();

        assert!(args.iter().any(|arg| arg == "Main.Core.MMU=true"));
    }
}
