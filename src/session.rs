use crate::config::ViewerLanguage;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::recorder::RecordingStats;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Manifest {
    pub session_id: String,
    pub started_at: u64,
    pub finished_at: Option<u64>,
    pub display_width: u32,
    pub display_height: u32,
    pub working_width: u32,
    pub working_height: u32,
    pub sampling_interval_ms: u64,
    pub block_width: u32,
    pub block_height: u32,
    pub keyframe_interval_ms: u64,
    pub sensitivity_mode: String,
    pub precheck_threshold: f32,
    pub block_difference_threshold: f32,
    pub changed_pixel_ratio_threshold: f32,
    pub stability_window: u32,
    pub compression_format: String,
    pub recorder_version: String,
    pub viewer_default_zoom: f32,
    pub viewer_overlay_enabled_by_default: bool,
    #[serde(default = "default_burn_in_enabled")]
    pub burn_in_enabled: bool,
    #[serde(default)]
    pub viewer_language: ViewerLanguage,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum SessionState {
    Running,
    Paused,
    Stopped,
}

#[derive(Debug, Clone, PartialEq)]
pub struct SessionLayout {
    root: PathBuf,
    manifest_path: PathBuf,
    status_path: PathBuf,
    stop_signal_path: PathBuf,
    pause_signal_path: PathBuf,
    keyframes_dir: PathBuf,
    patches_dir: PathBuf,
    index_dir: PathBuf,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SessionStatus {
    pub session_id: String,
    pub state: SessionState,
    pub recording: bool,
    pub stats: RecordingStats,
}

impl SessionLayout {
    pub fn new(output_dir: &Path, session_id: &str) -> Self {
        let root = output_dir.join("sessions").join(session_id);
        let manifest_path = root.join("manifest.json");
        let status_path = root.join("status.json");
        let stop_signal_path = root.join("stop.signal");
        let pause_signal_path = root.join("pause.signal");
        let keyframes_dir = root.join("keyframes");
        let patches_dir = root.join("patches");
        let index_dir = root.join("index");

        Self {
            root,
            manifest_path,
            status_path,
            stop_signal_path,
            pause_signal_path,
            keyframes_dir,
            patches_dir,
            index_dir,
        }
    }

    pub fn create_dirs(&self) -> std::io::Result<()> {
        std::fs::create_dir_all(&self.keyframes_dir)?;
        std::fs::create_dir_all(&self.patches_dir)?;
        std::fs::create_dir_all(&self.index_dir)?;
        Ok(())
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    pub fn manifest_path(&self) -> &Path {
        &self.manifest_path
    }

    pub fn status_path(&self) -> &Path {
        &self.status_path
    }

    pub fn stop_signal_path(&self) -> &Path {
        &self.stop_signal_path
    }

    pub fn pause_signal_path(&self) -> &Path {
        &self.pause_signal_path
    }

    pub fn keyframes_dir(&self) -> &Path {
        &self.keyframes_dir
    }

    pub fn patches_dir(&self) -> &Path {
        &self.patches_dir
    }

    pub fn index_dir(&self) -> &Path {
        &self.index_dir
    }
}

impl Manifest {
    pub fn load(path: &Path) -> Result<Self, std::io::Error> {
        let contents = std::fs::read_to_string(path)?;
        serde_json::from_str(&contents).map_err(std::io::Error::other)
    }
}

impl SessionStatus {
    pub fn load(path: &Path) -> Result<Self, std::io::Error> {
        let contents = std::fs::read_to_string(path)?;
        serde_json::from_str(&contents).map_err(std::io::Error::other)
    }
}

fn default_burn_in_enabled() -> bool {
    true
}
