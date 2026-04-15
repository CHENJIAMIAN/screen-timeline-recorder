use crate::config::ViewerLanguage;
use std::path::Path;
use std::{fs, path::PathBuf};

use serde::{Deserialize, Serialize};

use crate::diff::PatchRegion;
use crate::index::{IndexError, load_patch_index, nearest_keyframe, patch_entries_between};
use crate::reconstruct::{ReconstructError, Reconstructor};
use crate::session::{Manifest, SessionLayout, SessionState, SessionStatus};
use crate::storage::{StorageError, read_patch_region};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SessionResponse {
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
    pub burn_in_enabled: bool,
    pub viewer_language: ViewerLanguage,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SessionListEntry {
    pub session_id: String,
    pub started_at: u64,
    pub finished_at: Option<u64>,
    pub last_activity_at: u64,
    pub display_width: u32,
    pub display_height: u32,
    pub working_width: u32,
    pub working_height: u32,
    pub total_bytes: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status: Option<SessionStatusInfo>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SessionStatusInfo {
    pub state: SessionState,
    pub recording: bool,
    pub last_activity_at: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PatchMetadata {
    pub timestamp_ms: u64,
    pub sequence: u64,
    pub x: u32,
    pub y: u32,
    pub width: u32,
    pub height: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ActivityPoint {
    pub timestamp_ms: u64,
    pub patch_count: u64,
}

#[derive(Debug)]
pub enum ViewerApiError {
    Io(std::io::Error),
    Reconstruct(ReconstructError),
    Storage(StorageError),
    Index(IndexError),
    Png(png::EncodingError),
    MissingKeyframe(u64),
}

impl std::fmt::Display for ViewerApiError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Io(err) => write!(f, "viewer api io error: {err}"),
            Self::Reconstruct(err) => write!(f, "{err}"),
            Self::Storage(err) => write!(f, "{err}"),
            Self::Index(err) => write!(f, "{err}"),
            Self::Png(err) => write!(f, "png encoding error: {err}"),
            Self::MissingKeyframe(timestamp_ms) => {
                write!(f, "no keyframe available for timestamp {timestamp_ms}")
            }
        }
    }
}

impl std::error::Error for ViewerApiError {}

impl From<std::io::Error> for ViewerApiError {
    fn from(err: std::io::Error) -> Self {
        Self::Io(err)
    }
}

impl From<ReconstructError> for ViewerApiError {
    fn from(err: ReconstructError) -> Self {
        Self::Reconstruct(err)
    }
}

impl From<StorageError> for ViewerApiError {
    fn from(err: StorageError) -> Self {
        Self::Storage(err)
    }
}

impl From<IndexError> for ViewerApiError {
    fn from(err: IndexError) -> Self {
        Self::Index(err)
    }
}

impl From<png::EncodingError> for ViewerApiError {
    fn from(err: png::EncodingError) -> Self {
        Self::Png(err)
    }
}

pub fn get_session(output_dir: &Path, session_id: &str) -> Result<SessionResponse, ViewerApiError> {
    let layout = SessionLayout::new(output_dir, session_id);
    let manifest = Manifest::load(layout.manifest_path())?;
    Ok(SessionResponse {
        session_id: manifest.session_id,
        started_at: manifest.started_at,
        finished_at: manifest.finished_at,
        display_width: manifest.display_width,
        display_height: manifest.display_height,
        working_width: manifest.working_width,
        working_height: manifest.working_height,
        sampling_interval_ms: manifest.sampling_interval_ms,
        block_width: manifest.block_width,
        block_height: manifest.block_height,
        keyframe_interval_ms: manifest.keyframe_interval_ms,
        sensitivity_mode: manifest.sensitivity_mode,
        precheck_threshold: manifest.precheck_threshold,
        block_difference_threshold: manifest.block_difference_threshold,
        changed_pixel_ratio_threshold: manifest.changed_pixel_ratio_threshold,
        stability_window: manifest.stability_window,
        compression_format: manifest.compression_format,
        recorder_version: manifest.recorder_version,
        viewer_default_zoom: manifest.viewer_default_zoom,
        viewer_overlay_enabled_by_default: manifest.viewer_overlay_enabled_by_default,
        burn_in_enabled: manifest.burn_in_enabled,
        viewer_language: manifest.viewer_language,
    })
}

pub fn get_sessions(output_dir: &Path) -> Result<Vec<SessionListEntry>, ViewerApiError> {
    let sessions_root = output_dir.join("sessions");
    if !sessions_root.exists() {
        return Ok(Vec::new());
    }

    let mut sessions = Vec::new();
    for entry in fs::read_dir(&sessions_root)? {
        let entry = entry?;
        if !entry.file_type()?.is_dir() {
            continue;
        }

        let manifest_path: PathBuf = entry.path().join("manifest.json");
        if !manifest_path.exists() {
            continue;
        }

        let manifest = Manifest::load(&manifest_path)?;
        let layout = SessionLayout::new(output_dir, &manifest.session_id);
        let status = viewer_safe_status_info(&layout);
        let last_activity_at = status
            .as_ref()
            .map(|status| status.last_activity_at)
            .unwrap_or_else(|| manifest.finished_at.unwrap_or(manifest.started_at));
        sessions.push(SessionListEntry {
            session_id: manifest.session_id,
            started_at: manifest.started_at,
            finished_at: manifest.finished_at,
            last_activity_at,
            display_width: manifest.display_width,
            display_height: manifest.display_height,
            working_width: manifest.working_width,
            working_height: manifest.working_height,
            total_bytes: directory_size_bytes(&entry.path())?,
            status,
        });
    }

    sessions.sort_by(|left, right| {
        right
            .last_activity_at
            .cmp(&left.last_activity_at)
            .then_with(|| right.started_at.cmp(&left.started_at))
    });
    Ok(sessions)
}

pub fn get_status(output_dir: &Path, session_id: &str) -> Result<SessionStatus, ViewerApiError> {
    let layout = SessionLayout::new(output_dir, session_id);
    let status = SessionStatus::load(layout.status_path())?;
    Ok(viewer_safe_status(&layout, status))
}

pub fn get_activity(
    output_dir: &Path,
    session_id: &str,
) -> Result<Vec<ActivityPoint>, ViewerApiError> {
    let layout = SessionLayout::new(output_dir, session_id);
    let entries = load_patch_index(layout.index_dir())?;
    let mut points: Vec<ActivityPoint> = Vec::new();

    for entry in entries {
        if let Some(last) = points.last_mut()
            && last.timestamp_ms == entry.timestamp_ms
        {
            last.patch_count += 1;
            continue;
        }

        points.push(ActivityPoint {
            timestamp_ms: entry.timestamp_ms,
            patch_count: 1,
        });
    }

    Ok(points)
}

fn directory_size_bytes(path: &Path) -> Result<u64, std::io::Error> {
    let mut total = 0_u64;
    for entry in fs::read_dir(path)? {
        let entry = entry?;
        let metadata = entry.metadata()?;
        if metadata.is_dir() {
            total = total.saturating_add(directory_size_bytes(&entry.path())?);
        } else {
            total = total.saturating_add(metadata.len());
        }
    }
    Ok(total)
}

fn viewer_safe_status_info(layout: &SessionLayout) -> Option<SessionStatusInfo> {
    let status = SessionStatus::load(layout.status_path()).ok()?;
    let status = viewer_safe_status(layout, status);
    Some(SessionStatusInfo {
        state: status.state,
        recording: status.recording,
        last_activity_at: status.stats.finished_at.max(status.stats.started_at),
    })
}

fn viewer_safe_status(layout: &SessionLayout, mut status: SessionStatus) -> SessionStatus {
    if !matches!(status.state, SessionState::Running | SessionState::Paused) {
        return status;
    }

    if layout.stop_signal_path().exists() {
        status.state = SessionState::Stopped;
        status.recording = false;
        return status;
    }

    if layout.pause_signal_path().exists() {
        status.state = SessionState::Paused;
        status.recording = true;
        return status;
    }

    if status.stats.finished_at < status.stats.started_at {
        status.state = SessionState::Stopped;
        status.recording = false;
    }

    status
}

pub fn get_frame_png(
    output_dir: &Path,
    session_id: &str,
    timestamp_ms: u64,
) -> Result<Vec<u8>, ViewerApiError> {
    let reconstructor = Reconstructor::open(output_dir, session_id)?;
    let frame = reconstructor.reconstruct_at(timestamp_ms)?;
    encode_png(&frame)
}

pub fn get_patches(
    output_dir: &Path,
    session_id: &str,
    timestamp_ms: u64,
) -> Result<Vec<PatchMetadata>, ViewerApiError> {
    let layout = SessionLayout::new(output_dir, session_id);
    let manifest = Manifest::load(layout.manifest_path())?;
    let keyframe = nearest_keyframe(layout.index_dir(), timestamp_ms)?
        .ok_or(ViewerApiError::MissingKeyframe(timestamp_ms))?;
    let entries =
        patch_entries_between(layout.index_dir(), keyframe.timestamp_ms + 1, timestamp_ms)?;

    let mut patches = Vec::new();
    for entry in entries {
        let patch = read_patch_region(&layout, &entry, &manifest.compression_format)?;
        patches.push(PatchMetadata::from_patch(
            entry.timestamp_ms,
            entry.sequence,
            &patch,
        ));
    }
    Ok(patches)
}

fn encode_png(frame: &crate::frame::Frame) -> Result<Vec<u8>, ViewerApiError> {
    let mut buffer = Vec::new();
    let mut encoder = png::Encoder::new(&mut buffer, frame.width() as u32, frame.height() as u32);
    encoder.set_color(png::ColorType::Rgba);
    encoder.set_depth(png::BitDepth::Eight);
    {
        let mut writer = encoder.write_header()?;
        writer.write_image_data(frame.as_rgba())?;
    }
    Ok(buffer)
}

impl PatchMetadata {
    fn from_patch(timestamp_ms: u64, sequence: u64, patch: &PatchRegion) -> Self {
        Self {
            timestamp_ms,
            sequence,
            x: patch.x,
            y: patch.y,
            width: patch.width,
            height: patch.height,
        }
    }
}
