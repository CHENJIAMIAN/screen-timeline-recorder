use std::{fmt, fs, path::Path, path::PathBuf};

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum ViewerLanguage {
    Auto,
    En,
    Zh,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct RecorderConfig {
    #[serde(default = "default_output_dir")]
    pub output_dir: PathBuf,
    #[serde(default = "default_sampling_interval_ms")]
    pub sampling_interval_ms: u64,
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
        if self.sampling_interval_ms == 0 {
            return Err(ConfigError::InvalidValue(
                "sampling_interval_ms must be greater than zero",
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

        if let Some(max_sessions) = self.max_sessions
            && max_sessions == 0
        {
            return Err(ConfigError::InvalidValue(
                "max_sessions must be greater than zero",
            ));
        }

        if let Some(max_age_days) = self.max_age_days
            && max_age_days == 0
        {
            return Err(ConfigError::InvalidValue(
                "max_age_days must be greater than zero",
            ));
        }

        if let Some(max_total_bytes) = self.max_total_bytes
            && max_total_bytes == 0
        {
            return Err(ConfigError::InvalidValue(
                "max_total_bytes must be greater than zero",
            ));
        }

        Ok(())
    }

    pub fn with_output_dir(mut self, output_dir: PathBuf) -> Self {
        self.output_dir = output_dir;
        self
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
