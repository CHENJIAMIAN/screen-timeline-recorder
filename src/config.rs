use std::{fmt, fs, path::Path, path::PathBuf};

use serde::{Deserialize, Serialize};

use crate::storage::SessionDimensions;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum SensitivityMode {
    Conservative,
    Balanced,
    Detailed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum ViewerLanguage {
    Auto,
    En,
    Zh,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Thresholds {
    pub precheck_threshold: f32,
    pub block_difference_threshold: f32,
    pub changed_pixel_ratio_threshold: f32,
    pub stability_window: u32,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct RecorderConfig {
    #[serde(default = "default_output_dir")]
    pub output_dir: PathBuf,
    #[serde(default = "default_sampling_interval_ms")]
    pub sampling_interval_ms: u64,
    #[serde(default = "default_block_width")]
    pub block_width: u32,
    #[serde(default = "default_block_height")]
    pub block_height: u32,
    #[serde(default = "default_keyframe_interval_ms")]
    pub keyframe_interval_ms: u64,
    #[serde(default)]
    pub sensitivity_mode: SensitivityMode,
    #[serde(default = "default_working_scale")]
    pub working_scale: f32,
    #[serde(default = "default_viewer_default_zoom")]
    pub viewer_default_zoom: f32,
    #[serde(default = "default_viewer_overlay_enabled_by_default")]
    pub viewer_overlay_enabled_by_default: bool,
    #[serde(default = "default_burn_in_enabled")]
    pub burn_in_enabled: bool,
    #[serde(default)]
    pub viewer_language: ViewerLanguage,
    #[serde(default = "default_max_sessions")]
    pub max_sessions: Option<u32>,
    #[serde(default = "default_max_age_days")]
    pub max_age_days: Option<u32>,
    #[serde(default = "default_max_total_bytes")]
    pub max_total_bytes: Option<u64>,
}

impl Default for RecorderConfig {
    fn default() -> Self {
        Self {
            output_dir: default_output_dir(),
            sampling_interval_ms: default_sampling_interval_ms(),
            block_width: default_block_width(),
            block_height: default_block_height(),
            keyframe_interval_ms: default_keyframe_interval_ms(),
            sensitivity_mode: SensitivityMode::default(),
            working_scale: default_working_scale(),
            viewer_default_zoom: default_viewer_default_zoom(),
            viewer_overlay_enabled_by_default: default_viewer_overlay_enabled_by_default(),
            burn_in_enabled: default_burn_in_enabled(),
            viewer_language: ViewerLanguage::default(),
            max_sessions: default_max_sessions(),
            max_age_days: default_max_age_days(),
            max_total_bytes: default_max_total_bytes(),
        }
    }
}

impl Default for SensitivityMode {
    fn default() -> Self {
        Self::Balanced
    }
}

impl Default for ViewerLanguage {
    fn default() -> Self {
        Self::Auto
    }
}

impl RecorderConfig {
    pub fn from_path(path: &Path) -> Result<Self, ConfigError> {
        let contents = fs::read_to_string(path).map_err(ConfigError::Io)?;
        toml::from_str(&contents).map_err(ConfigError::Toml)
    }

    pub fn validate(&self) -> Result<(), ConfigError> {
        if self.block_width == 0 || self.block_height == 0 {
            return Err(ConfigError::InvalidValue(
                "block size must be greater than zero",
            ));
        }

        if self.keyframe_interval_ms == 0 {
            return Err(ConfigError::InvalidValue(
                "keyframe interval must be greater than zero",
            ));
        }

        if !(0.0 < self.working_scale && self.working_scale <= 1.0) {
            return Err(ConfigError::InvalidValue(
                "working_scale must be in the range (0.0, 1.0]",
            ));
        }

        if self.viewer_default_zoom <= 0.0 {
            return Err(ConfigError::InvalidValue(
                "viewer_default_zoom must be greater than zero",
            ));
        }

        if let Some(max_sessions) = self.max_sessions {
            if max_sessions == 0 {
                return Err(ConfigError::InvalidValue(
                    "max_sessions must be greater than zero",
                ));
            }
        }

        if let Some(max_age_days) = self.max_age_days {
            if max_age_days == 0 {
                return Err(ConfigError::InvalidValue(
                    "max_age_days must be greater than zero",
                ));
            }
        }

        if let Some(max_total_bytes) = self.max_total_bytes {
            if max_total_bytes == 0 {
                return Err(ConfigError::InvalidValue(
                    "max_total_bytes must be greater than zero",
                ));
            }
        }

        Ok(())
    }

    pub fn with_output_dir(mut self, output_dir: PathBuf) -> Self {
        self.output_dir = output_dir;
        self
    }

    pub fn thresholds(&self) -> Thresholds {
        match self.sensitivity_mode {
            SensitivityMode::Conservative => Thresholds {
                precheck_threshold: 0.02,
                block_difference_threshold: 0.08,
                changed_pixel_ratio_threshold: 0.15,
                stability_window: 3,
            },
            SensitivityMode::Balanced => Thresholds {
                precheck_threshold: 0.01,
                block_difference_threshold: 0.05,
                changed_pixel_ratio_threshold: 0.0,
                stability_window: 2,
            },
            SensitivityMode::Detailed => Thresholds {
                precheck_threshold: 0.005,
                block_difference_threshold: 0.02,
                changed_pixel_ratio_threshold: 0.0,
                stability_window: 1,
            },
        }
    }

    pub fn session_dimensions(&self, display_width: u32, display_height: u32) -> SessionDimensions {
        let working_width = ((display_width as f32) * self.working_scale)
            .round()
            .clamp(1.0, display_width as f32) as u32;
        let working_height = ((display_height as f32) * self.working_scale)
            .round()
            .clamp(1.0, display_height as f32) as u32;

        SessionDimensions {
            display_width,
            display_height,
            working_width,
            working_height,
        }
    }
}

#[derive(Debug)]
pub enum ConfigError {
    Io(std::io::Error),
    Toml(toml::de::Error),
    InvalidValue(&'static str),
    Settings(String),
}

impl fmt::Display for ConfigError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(err) => write!(f, "failed to read config file: {err}"),
            Self::Toml(err) => write!(f, "failed to parse config file: {err}"),
            Self::InvalidValue(message) => write!(f, "invalid configuration: {message}"),
            Self::Settings(message) => {
                write!(f, "failed to load saved recording settings: {message}")
            }
        }
    }
}

impl std::error::Error for ConfigError {}

fn default_output_dir() -> PathBuf {
    PathBuf::from("output")
}

fn default_sampling_interval_ms() -> u64 {
    100
}

fn default_block_width() -> u32 {
    16
}

fn default_block_height() -> u32 {
    16
}

fn default_keyframe_interval_ms() -> u64 {
    30_000
}

fn default_working_scale() -> f32 {
    1.0
}

fn default_viewer_default_zoom() -> f32 {
    1.0
}

fn default_viewer_overlay_enabled_by_default() -> bool {
    true
}

fn default_burn_in_enabled() -> bool {
    true
}

fn default_max_sessions() -> Option<u32> {
    None
}

fn default_max_age_days() -> Option<u32> {
    None
}

fn default_max_total_bytes() -> Option<u64> {
    None
}
