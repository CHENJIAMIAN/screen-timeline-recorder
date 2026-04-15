use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

use crate::config::{RecorderConfig, SensitivityMode};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RecordingSettings {
    pub sampling_interval_ms: u64,
    pub block_width: u32,
    pub block_height: u32,
    pub keyframe_interval_ms: u64,
    pub sensitivity_mode: SensitivityMode,
    pub working_scale: f32,
    #[serde(default = "default_burn_in_enabled")]
    pub burn_in_enabled: bool,
}

#[derive(Debug)]
pub enum RecordingSettingsError {
    Io(std::io::Error),
    Json(serde_json::Error),
    Invalid(String),
}

impl std::fmt::Display for RecordingSettingsError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Io(err) => write!(f, "recording settings io failed: {err}"),
            Self::Json(err) => write!(f, "recording settings json failed: {err}"),
            Self::Invalid(message) => write!(f, "{message}"),
        }
    }
}

impl std::error::Error for RecordingSettingsError {}

impl From<std::io::Error> for RecordingSettingsError {
    fn from(err: std::io::Error) -> Self {
        Self::Io(err)
    }
}

impl From<serde_json::Error> for RecordingSettingsError {
    fn from(err: serde_json::Error) -> Self {
        Self::Json(err)
    }
}

impl RecordingSettings {
    pub fn defaults() -> Self {
        Self::from_config(&RecorderConfig::default())
    }

    pub fn from_config(config: &RecorderConfig) -> Self {
        Self {
            sampling_interval_ms: config.sampling_interval_ms,
            block_width: config.block_width,
            block_height: config.block_height,
            keyframe_interval_ms: config.keyframe_interval_ms,
            sensitivity_mode: config.sensitivity_mode,
            working_scale: config.working_scale,
            burn_in_enabled: config.burn_in_enabled,
        }
    }

    pub fn apply_to_config(&self, config: &mut RecorderConfig) {
        config.sampling_interval_ms = self.sampling_interval_ms;
        config.block_width = self.block_width;
        config.block_height = self.block_height;
        config.keyframe_interval_ms = self.keyframe_interval_ms;
        config.sensitivity_mode = self.sensitivity_mode;
        config.working_scale = self.working_scale;
        config.burn_in_enabled = self.burn_in_enabled;
    }

    pub fn validate(&self) -> Result<(), RecordingSettingsError> {
        let mut config = RecorderConfig::default();
        self.apply_to_config(&mut config);
        config
            .validate()
            .map_err(|err| RecordingSettingsError::Invalid(err.to_string()))
    }
}

fn default_burn_in_enabled() -> bool {
    true
}

pub fn load_recording_settings(
    output_dir: &Path,
) -> Result<RecordingSettings, RecordingSettingsError> {
    let path = settings_path(output_dir);
    if !path.exists() {
        return Ok(RecordingSettings::defaults());
    }

    let raw = std::fs::read_to_string(path)?;
    let settings: RecordingSettings = serde_json::from_str(&raw)?;
    settings.validate()?;
    Ok(settings)
}

pub fn save_recording_settings(
    output_dir: &Path,
    settings: &RecordingSettings,
) -> Result<(), RecordingSettingsError> {
    settings.validate()?;
    std::fs::create_dir_all(output_dir)?;
    let body = serde_json::to_string_pretty(settings)?;
    std::fs::write(settings_path(output_dir), body)?;
    Ok(())
}

pub fn apply_recording_settings(
    output_dir: &Path,
    settings: &RecordingSettings,
) -> Result<RecordingSettings, RecordingSettingsError> {
    save_recording_settings(output_dir, settings)?;
    load_recording_settings(output_dir)
}

fn settings_path(output_dir: &Path) -> PathBuf {
    output_dir.join("recording-settings.json")
}
