use crate::config::ViewerLanguage;
use crate::session::{RecordingFormat, SessionLayout, SessionState, SessionStatus};
use crate::video_session::{VideoSegmentEntry, VideoSessionManifest, load_video_segment_index};
use serde::{Deserialize, Serialize};
use std::path::Path;
use std::{fs, path::PathBuf};
use std::time::UNIX_EPOCH;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SessionResponse {
    pub session_id: String,
    pub started_at: u64,
    pub finished_at: Option<u64>,
    pub recording_format: RecordingFormat,
    pub display_width: u32,
    pub display_height: u32,
    pub working_width: u32,
    pub working_height: u32,
    pub sampling_interval_ms: u64,
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
    pub recording_format: RecordingFormat,
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

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ActivityPoint {
    pub timestamp_ms: u64,
    pub patch_count: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct VideoSegmentResponse {
    pub sequence: u64,
    pub started_at: u64,
    pub finished_at: Option<u64>,
    pub relative_path: String,
    pub bytes: u64,
}

#[derive(Debug)]
pub enum ViewerApiError {
    Io(std::io::Error),
}

impl std::fmt::Display for ViewerApiError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Io(err) => write!(f, "viewer api io error: {err}"),
        }
    }
}

impl std::error::Error for ViewerApiError {}

impl From<std::io::Error> for ViewerApiError {
    fn from(err: std::io::Error) -> Self {
        Self::Io(err)
    }
}

pub fn get_session(output_dir: &Path, session_id: &str) -> Result<SessionResponse, ViewerApiError> {
    let layout = SessionLayout::new(output_dir, session_id);
    load_session_response(layout.manifest_path())
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

        let session = load_session_response(&manifest_path)?;
        let layout = SessionLayout::new(output_dir, &session.session_id);
        let status = viewer_safe_status_info(&layout);
        let finished_at = resolved_session_finished_at(&session, &layout, status.as_ref());
        let last_activity_at = status
            .as_ref()
            .map(|status| status.last_activity_at)
            .unwrap_or_else(|| finished_at.unwrap_or(session.started_at));
        sessions.push(SessionListEntry {
            session_id: session.session_id,
            started_at: session.started_at,
            finished_at,
            last_activity_at,
            recording_format: session.recording_format,
            display_width: session.display_width,
            display_height: session.display_height,
            working_width: session.working_width,
            working_height: session.working_height,
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

fn load_session_response(path: &Path) -> Result<SessionResponse, ViewerApiError> {
    let manifest = VideoSessionManifest::load(path)?;
    Ok(SessionResponse {
        session_id: manifest.session_id,
        started_at: manifest.started_at,
        finished_at: manifest.finished_at,
        recording_format: manifest.recording_format,
        display_width: manifest.display_width,
        display_height: manifest.display_height,
        working_width: manifest.video_width,
        working_height: manifest.video_height,
        sampling_interval_ms: manifest.segment_duration_ms,
        compression_format: manifest.video_codec.clone(),
        recorder_version: manifest.recorder_version,
        viewer_default_zoom: manifest.viewer_default_zoom,
        viewer_overlay_enabled_by_default: manifest.viewer_overlay_enabled_by_default,
        burn_in_enabled: manifest.burn_in_enabled,
        viewer_language: manifest.viewer_language,
    })
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
    let segments = get_video_segments(output_dir, session_id)?;
    Ok(segments
        .into_iter()
        .map(|segment| ActivityPoint {
            timestamp_ms: segment.started_at,
            patch_count: 1,
        })
        .collect())
}

pub fn get_video_segments(
    output_dir: &Path,
    session_id: &str,
) -> Result<Vec<VideoSegmentResponse>, ViewerApiError> {
    let layout = SessionLayout::new(output_dir, session_id);
    let manifest = VideoSessionManifest::load(layout.manifest_path())?;
    let mut entries = load_video_segment_index(&layout.index_dir().join("segments.jsonl"))?;
    if entries.is_empty() {
        entries = rebuild_video_segments_from_disk(&layout, &manifest)?;
    }
    Ok(entries
        .into_iter()
        .map(|entry: VideoSegmentEntry| VideoSegmentResponse {
            sequence: entry.sequence,
            started_at: entry.started_at,
            finished_at: entry.finished_at,
            relative_path: entry.relative_path,
            bytes: entry.bytes,
        })
        .collect())
}

fn rebuild_video_segments_from_disk(
    layout: &SessionLayout,
    manifest: &VideoSessionManifest,
) -> Result<Vec<VideoSegmentEntry>, ViewerApiError> {
    let mut files: Vec<_> = fs::read_dir(layout.segments_dir())?
        .filter_map(|entry| entry.ok())
        .filter(|entry| entry.file_type().map(|kind| kind.is_file()).unwrap_or(false))
        .collect();
    files.sort_by_key(|entry| entry.file_name());

    let mut entries = Vec::new();
    for (index, entry) in files.into_iter().enumerate() {
        let metadata = entry.metadata()?;
        let started_at = manifest.started_at + index as u64 * manifest.segment_duration_ms;
        let finished_at = Some(started_at + manifest.segment_duration_ms);
        entries.push(VideoSegmentEntry {
            sequence: index as u64,
            started_at,
            finished_at,
            relative_path: format!("segments/{}", entry.file_name().to_string_lossy()),
            bytes: metadata.len(),
        });
    }
    Ok(entries)
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

fn resolved_session_finished_at(
    session: &SessionResponse,
    layout: &SessionLayout,
    status: Option<&SessionStatusInfo>,
) -> Option<u64> {
    if let Some(finished_at) = session.finished_at
        && finished_at > session.started_at
    {
        return Some(finished_at);
    }

    if status.is_some_and(|entry| entry.recording) {
        return None;
    }

    infer_video_finished_at(layout, session.started_at)
}

fn infer_video_finished_at(layout: &SessionLayout, started_at: u64) -> Option<u64> {
    let segments_dir = layout.segments_dir();
    let mut latest: Option<u64> = None;
    for entry in fs::read_dir(segments_dir).ok()? {
        let entry = entry.ok()?;
        let metadata = entry.metadata().ok()?;
        if !metadata.is_file() {
            continue;
        }
        let modified = metadata.modified().ok()?;
        let timestamp_ms = modified.duration_since(UNIX_EPOCH).ok()?.as_millis() as u64;
        if timestamp_ms > started_at {
            latest = Some(latest.map_or(timestamp_ms, |current| current.max(timestamp_ms)));
        }
    }
    latest
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
