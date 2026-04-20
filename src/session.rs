use crate::recording_stats::RecordingStats;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum RecordingFormat {
    VideoSegments,
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
    segments_dir: PathBuf,
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
        let segments_dir = root.join("segments");
        let index_dir = root.join("index");

        Self {
            root,
            manifest_path,
            status_path,
            stop_signal_path,
            pause_signal_path,
            segments_dir,
            index_dir,
        }
    }

    pub fn create_video_dirs(&self) -> std::io::Result<()> {
        std::fs::create_dir_all(&self.root)?;
        std::fs::create_dir_all(&self.segments_dir)?;
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

    pub fn segments_dir(&self) -> &Path {
        &self.segments_dir
    }

    pub fn index_dir(&self) -> &Path {
        &self.index_dir
    }
}

impl SessionStatus {
    pub fn load(path: &Path) -> Result<Self, std::io::Error> {
        let contents = std::fs::read_to_string(path)?;
        serde_json::from_str(&contents).map_err(std::io::Error::other)
    }
}

impl Default for RecordingFormat {
    fn default() -> Self {
        Self::VideoSegments
    }
}
