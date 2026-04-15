use serde::{Deserialize, Serialize};
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
        );

        assert!(command.contains("desktop --background --autorun-record"));
        assert!(command.contains(&temp_dir.path().display().to_string()));
        assert!(command.contains("timeout /t 15"));
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
    let task_command = scheduled_task_command(&exe_path, settings);

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

fn scheduled_task_command(exe_path: &Path, settings: &AutostartSettings) -> String {
    let desktop_command = format!(
        "\"{}\" desktop --background --autorun-record --output-dir \"{}\"",
        exe_path.display(),
        settings.output_dir.display()
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

#[cfg(not(windows))]
fn register_windows_task(_settings: &AutostartSettings) -> Result<(), AutostartError> {
    Ok(())
}

#[cfg(not(windows))]
fn unregister_windows_task() -> Result<(), AutostartError> {
    Ok(())
}
