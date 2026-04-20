use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

pub const AUTOSTART_TASK_NAME: &str = "ScreenTimelineRecorder_Autostart";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AutostartSettings {
    pub enabled: bool,
    pub start_on_login: bool,
    pub delay_seconds: u32,
    pub output_dir: PathBuf,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AutostartStatus {
    pub supported: bool,
    pub task_name: String,
    pub task_registered: bool,
    pub settings: AutostartSettings,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
}

#[derive(Debug)]
pub enum AutostartError {
    Io(std::io::Error),
    Json(serde_json::Error),
    Invalid(String),
    Command(String),
}

impl std::fmt::Display for AutostartError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Io(err) => write!(f, "autostart io failed: {err}"),
            Self::Json(err) => write!(f, "autostart json failed: {err}"),
            Self::Invalid(message) => write!(f, "{message}"),
            Self::Command(message) => write!(f, "{message}"),
        }
    }
}

impl std::error::Error for AutostartError {}

impl From<std::io::Error> for AutostartError {
    fn from(err: std::io::Error) -> Self {
        Self::Io(err)
    }
}

impl From<serde_json::Error> for AutostartError {
    fn from(err: serde_json::Error) -> Self {
        Self::Json(err)
    }
}

impl AutostartSettings {
    pub fn default_for(output_dir: &Path) -> Self {
        Self {
            enabled: false,
            start_on_login: true,
            delay_seconds: 30,
            output_dir: output_dir.to_path_buf(),
        }
    }

    pub fn validate(&self) -> Result<(), AutostartError> {
        if self.delay_seconds > 3600 {
            return Err(AutostartError::Invalid(
                "delay_seconds must be 3600 or less".to_string(),
            ));
        }
        if self.output_dir.as_os_str().is_empty() {
            return Err(AutostartError::Invalid(
                "output_dir must not be empty".to_string(),
            ));
        }
        Ok(())
    }
}

pub fn load_autostart_settings(output_dir: &Path) -> Result<AutostartSettings, AutostartError> {
    let settings_path = settings_path(output_dir);
    if !settings_path.exists() {
        return Ok(AutostartSettings::default_for(output_dir));
    }

    let raw = std::fs::read_to_string(settings_path)?;
    let mut settings: AutostartSettings = serde_json::from_str(&raw)?;
    if settings.output_dir.as_os_str().is_empty() {
        settings.output_dir = output_dir.to_path_buf();
    }
    Ok(settings)
}

pub fn save_autostart_settings(
    output_dir: &Path,
    settings: &AutostartSettings,
) -> Result<(), AutostartError> {
    std::fs::create_dir_all(output_dir)?;
    let body = serde_json::to_string_pretty(settings)?;
    std::fs::write(settings_path(output_dir), body)?;
    Ok(())
}

pub fn get_autostart_status(output_dir: &Path) -> Result<AutostartStatus, AutostartError> {
    let settings = load_autostart_settings(output_dir)?;
    Ok(AutostartStatus {
        supported: cfg!(windows),
        task_name: AUTOSTART_TASK_NAME.to_string(),
        task_registered: task_registered(AUTOSTART_TASK_NAME),
        settings,
        note: None,
    })
}

pub fn apply_autostart_settings(
    output_dir: &Path,
    settings: &AutostartSettings,
) -> Result<AutostartStatus, AutostartError> {
    settings.validate()?;
    #[cfg(windows)]
    {
        if settings.enabled {
            register_windows_task(settings)?;
        } else {
            unregister_windows_task()?;
        }
        save_autostart_settings(output_dir, settings)?;
    }

    get_autostart_status(output_dir)
}

fn settings_path(output_dir: &Path) -> PathBuf {
    output_dir.join("autostart.json")
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[cfg(not(windows))]
    #[test]
    fn apply_autostart_settings_does_not_persist_when_unsupported() {
        let temp_dir = tempdir().expect("tempdir");
        let settings = AutostartSettings {
            enabled: true,
            start_on_login: true,
            delay_seconds: 10,
            output_dir: temp_dir.path().to_path_buf(),
        };

        let status = apply_autostart_settings(temp_dir.path(), &settings)
            .expect("apply_autostart_settings should succeed on non-windows");

        assert_eq!(
            status.settings,
            AutostartSettings::default_for(temp_dir.path())
        );
        assert!(!settings_path(temp_dir.path()).exists());
    }

    #[test]
    fn scheduled_task_command_uses_desktop_background_mode() {
        let temp_dir = tempdir().expect("tempdir");
        let settings = AutostartSettings {
            enabled: true,
            start_on_login: true,
            delay_seconds: 15,
            output_dir: temp_dir.path().to_path_buf(),
        };

        let command = scheduled_task_command(
            Path::new("C:\\app\\screen-timeline-recorder.exe"),
            &settings,
            temp_dir.path(),
        );

        assert!(command.contains("desktop --background --autorun-record"));
        assert!(command.contains(&temp_dir.path().display().to_string()));
        assert!(command.contains("timeout /t 15"));
    }

    #[test]
    fn prepare_autostart_runtime_copies_debug_build_to_dist_desktop() {
        let temp_dir = tempdir().expect("tempdir");
        let repo_root = temp_dir.path();
        let exe_path = repo_root
            .join("target")
            .join("debug")
            .join("screen-timeline-recorder.exe");
        let viewer_file = repo_root.join("viewer").join("index.html");
        let ffmpeg_file = repo_root.join("tools").join("ffmpeg").join("ffmpeg.exe");

        fs::create_dir_all(exe_path.parent().expect("exe dir")).expect("create exe dir");
        fs::create_dir_all(viewer_file.parent().expect("viewer dir")).expect("create viewer dir");
        fs::create_dir_all(ffmpeg_file.parent().expect("ffmpeg dir")).expect("create ffmpeg dir");
        fs::write(&exe_path, b"exe").expect("write exe");
        fs::write(&viewer_file, b"viewer").expect("write viewer");
        fs::write(&ffmpeg_file, b"ffmpeg").expect("write ffmpeg");

        let staged = prepare_autostart_runtime(&exe_path, repo_root).expect("stage runtime");

        assert_eq!(
            staged,
            repo_root
                .join("dist")
                .join("desktop")
                .join("screen-timeline-recorder.exe")
        );
        assert!(repo_root.join("dist").join("desktop").join("viewer").join("index.html").is_file());
        assert!(repo_root.join("dist").join("desktop").join("ffmpeg").join("ffmpeg.exe").is_file());
    }
}

fn task_registered(task_name: &str) -> bool {
    #[cfg(windows)]
    {
        Command::new("schtasks")
            .args(["/Query", "/TN", task_name])
            .output()
            .map(|output| output.status.success())
            .unwrap_or(false)
    }
    #[cfg(not(windows))]
    {
        let _ = task_name;
        false
    }
}

#[cfg(windows)]
fn register_windows_task(settings: &AutostartSettings) -> Result<(), AutostartError> {
    let exe_path = std::env::current_exe()?;
    let repo_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let runtime_exe = prepare_autostart_runtime(&exe_path, &repo_root)?;
    let task_command = scheduled_task_command(&runtime_exe, settings, &repo_root);

    let output = Command::new("schtasks")
        .args([
            "/Create",
            "/F",
            "/SC",
            "ONLOGON",
            "/TN",
            AUTOSTART_TASK_NAME,
            "/TR",
            &task_command,
        ])
        .output()?;

    if output.status.success() {
        Ok(())
    } else {
        Err(AutostartError::Command(format!(
            "failed to create scheduled task: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        )))
    }
}

#[cfg(windows)]
fn unregister_windows_task() -> Result<(), AutostartError> {
    if !task_registered(AUTOSTART_TASK_NAME) {
        return Ok(());
    }

    let output = Command::new("schtasks")
        .args(["/Delete", "/F", "/TN", AUTOSTART_TASK_NAME])
        .output()?;
    if output.status.success() {
        Ok(())
    } else {
        Err(AutostartError::Command(format!(
            "failed to delete scheduled task: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        )))
    }
}

fn scheduled_task_command(exe_path: &Path, settings: &AutostartSettings, base_dir: &Path) -> String {
    let output_dir = absolutize_path(&settings.output_dir, base_dir);
    let desktop_command = format!(
        "\"{}\" desktop --background --autorun-record --output-dir \"{}\"",
        exe_path.display(),
        output_dir.display()
    );
    if settings.delay_seconds > 0 {
        format!(
            "cmd.exe /c timeout /t {} /nobreak >nul && {}",
            settings.delay_seconds, desktop_command
        )
    } else {
        desktop_command
    }
}

fn absolutize_path(path: &Path, base_dir: &Path) -> PathBuf {
    if path.is_absolute() {
        path.to_path_buf()
    } else {
        base_dir.join(path)
    }
}

fn prepare_autostart_runtime(current_exe: &Path, repo_root: &Path) -> Result<PathBuf, AutostartError> {
    if !should_stage_autostart_runtime(current_exe) {
        return Ok(current_exe.to_path_buf());
    }

    let runtime_dir = repo_root.join("dist").join("desktop");
    fs::create_dir_all(&runtime_dir)?;

    let runtime_exe = runtime_dir.join(
        current_exe
            .file_name()
            .ok_or_else(|| AutostartError::Invalid("current exe has no file name".to_string()))?,
    );
    fs::copy(current_exe, &runtime_exe)?;

    copy_dir_contents(&repo_root.join("viewer"), &runtime_dir.join("viewer"))?;

    let icon_path = repo_root.join("icons").join("icon.ico");
    if icon_path.is_file() {
        copy_file_to_dir(&icon_path, &runtime_dir.join("icons"))?;
    }

    let ffmpeg_dir = repo_root.join("tools").join("ffmpeg");
    if ffmpeg_dir.is_dir() {
        copy_dir_contents(&ffmpeg_dir, &runtime_dir.join("ffmpeg"))?;
    }

    Ok(runtime_exe)
}

fn should_stage_autostart_runtime(current_exe: &Path) -> bool {
    let path = current_exe.to_string_lossy().replace('\\', "/").to_lowercase();
    path.contains("/target/debug/") || path.contains("/target/release/")
}

fn copy_file_to_dir(source: &Path, dest_dir: &Path) -> Result<(), AutostartError> {
    fs::create_dir_all(dest_dir)?;
    let file_name = source
        .file_name()
        .ok_or_else(|| AutostartError::Invalid(format!("missing file name for {}", source.display())))?;
    fs::copy(source, dest_dir.join(file_name))?;
    Ok(())
}

fn copy_dir_contents(source_dir: &Path, dest_dir: &Path) -> Result<(), AutostartError> {
    if !source_dir.exists() {
        return Ok(());
    }

    fs::create_dir_all(dest_dir)?;
    for entry in fs::read_dir(source_dir)? {
        let entry = entry?;
        let source_path = entry.path();
        let dest_path = dest_dir.join(entry.file_name());
        if entry.file_type()?.is_dir() {
            copy_dir_contents(&source_path, &dest_path)?;
        } else {
            fs::copy(source_path, dest_path)?;
        }
    }
    Ok(())
}

#[cfg(not(windows))]
fn register_windows_task(_settings: &AutostartSettings) -> Result<(), AutostartError> {
    Ok(())
}

#[cfg(not(windows))]
fn unregister_windows_task() -> Result<(), AutostartError> {
    Ok(())
}
