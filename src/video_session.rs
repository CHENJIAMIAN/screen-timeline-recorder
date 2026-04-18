use std::{
    fs::OpenOptions,
    io::{BufRead, BufReader, Write},
    path::Path,
};

use serde::{Deserialize, Serialize};

use crate::{config::ViewerLanguage, session::RecordingFormat};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct VideoSessionManifest {
    pub session_id: String,
    pub started_at: u64,
    pub finished_at: Option<u64>,
    pub display_width: u32,
    pub display_height: u32,
    pub video_width: u32,
    pub video_height: u32,
    pub recording_format: RecordingFormat,
    pub segment_duration_ms: u64,
    pub video_codec: String,
    pub recorder_version: String,
    pub viewer_default_zoom: f32,
    pub viewer_overlay_enabled_by_default: bool,
    pub burn_in_enabled: bool,
    pub viewer_language: ViewerLanguage,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct VideoSegmentEntry {
    pub sequence: u64,
    pub started_at: u64,
    pub finished_at: Option<u64>,
    pub relative_path: String,
    pub bytes: u64,
}

pub fn append_video_segment_index(
    path: &Path,
    entry: &VideoSegmentEntry,
) -> Result<(), std::io::Error> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let mut file = OpenOptions::new().create(true).append(true).open(path)?;
    let line = serde_json::to_string(entry).map_err(std::io::Error::other)?;
    writeln!(file, "{line}")?;
    Ok(())
}

pub fn load_video_segment_index(path: &Path) -> Result<Vec<VideoSegmentEntry>, std::io::Error> {
    if !path.exists() {
        return Ok(Vec::new());
    }

    let file = OpenOptions::new().read(true).open(path)?;
    let reader = BufReader::new(file);
    let mut entries = Vec::new();
    for line in reader.lines() {
        let line = line?;
        if line.trim().is_empty() {
            continue;
        }
        entries.push(serde_json::from_str(&line).map_err(std::io::Error::other)?);
    }
    Ok(entries)
}

impl VideoSessionManifest {
    pub fn load(path: &Path) -> Result<Self, std::io::Error> {
        let contents = std::fs::read_to_string(path)?;
        serde_json::from_str(&contents).map_err(std::io::Error::other)
    }
}
