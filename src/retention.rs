use std::fs;
use std::path::{Path, PathBuf};

use crate::video_session::VideoSessionManifest;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RetentionReport {
    pub removed_sessions: Vec<String>,
}

#[derive(Debug)]
pub enum RetentionError {
    Io(std::io::Error),
}

impl std::fmt::Display for RetentionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Io(err) => write!(f, "retention cleanup failed: {err}"),
        }
    }
}

impl std::error::Error for RetentionError {}

impl From<std::io::Error> for RetentionError {
    fn from(err: std::io::Error) -> Self {
        Self::Io(err)
    }
}

pub fn enforce_max_sessions(
    output_dir: &Path,
    max_sessions: Option<usize>,
) -> Result<RetentionReport, RetentionError> {
    enforce_retention(output_dir, max_sessions, None, None, u64::MAX)
}

pub fn enforce_retention(
    output_dir: &Path,
    max_sessions: Option<usize>,
    max_age_days: Option<u32>,
    max_total_bytes: Option<u64>,
    now_timestamp_ms: u64,
) -> Result<RetentionReport, RetentionError> {
    if max_sessions.is_none() && max_age_days.is_none() && max_total_bytes.is_none() {
        return Ok(RetentionReport {
            removed_sessions: Vec::new(),
        });
    }

    let sessions_root = output_dir.join("sessions");
    if !sessions_root.exists() {
        return Ok(RetentionReport {
            removed_sessions: Vec::new(),
        });
    }

    let mut sessions = Vec::new();
    for entry in fs::read_dir(&sessions_root)? {
        let entry = entry?;
        if !entry.file_type()?.is_dir() {
            continue;
        }

        let root = entry.path();
        let manifest_path = root.join("manifest.json");
        if !manifest_path.exists() {
            continue;
        }

        let manifest = match VideoSessionManifest::load(&manifest_path) {
            Ok(manifest) => manifest,
            Err(_) => continue,
        };

        sessions.push(TrackedSession {
            session_id: manifest.session_id,
            started_at: manifest.started_at,
            total_bytes: directory_size_bytes(&root)?,
            root,
        });
    }

    sessions.sort_by(|left, right| right.started_at.cmp(&left.started_at));
    let cutoff = max_age_days.and_then(|days| retention_cutoff_ms(now_timestamp_ms, days));
    let mut retained_total_bytes: u64 = sessions.iter().map(|session| session.total_bytes).sum();

    let mut removed_sessions = Vec::new();
    for (index, session) in sessions.into_iter().enumerate() {
        let exceeds_count = max_sessions.is_some_and(|limit| index >= limit);
        let exceeds_age = cutoff.is_some_and(|cutoff_ms| session.started_at < cutoff_ms);
        let exceeds_total =
            max_total_bytes.is_some_and(|limit| retained_total_bytes > limit && index > 0);
        if exceeds_count || exceeds_age || exceeds_total {
            fs::remove_dir_all(&session.root)?;
            retained_total_bytes = retained_total_bytes.saturating_sub(session.total_bytes);
            removed_sessions.push(session.session_id);
        }
    }

    Ok(RetentionReport { removed_sessions })
}

#[derive(Debug)]
struct TrackedSession {
    session_id: String,
    started_at: u64,
    total_bytes: u64,
    root: PathBuf,
}

fn retention_cutoff_ms(now_timestamp_ms: u64, max_age_days: u32) -> Option<u64> {
    let days_ms = u64::from(max_age_days).checked_mul(24 * 60 * 60 * 1_000)?;
    now_timestamp_ms.checked_sub(days_ms)
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
