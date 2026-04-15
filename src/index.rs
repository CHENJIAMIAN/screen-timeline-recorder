use std::fs::OpenOptions;
use std::io::{BufRead, BufReader, Write};
use std::path::Path;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct KeyframeIndexEntry {
    pub timestamp_ms: u64,
    pub path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PatchIndexEntry {
    pub timestamp_ms: u64,
    pub sequence: u64,
    pub path: String,
}

#[derive(Debug)]
pub enum IndexError {
    Io(std::io::Error),
    Json(serde_json::Error),
}

impl std::fmt::Display for IndexError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Io(err) => write!(f, "index io error: {err}"),
            Self::Json(err) => write!(f, "index json error: {err}"),
        }
    }
}

impl std::error::Error for IndexError {}

impl From<std::io::Error> for IndexError {
    fn from(err: std::io::Error) -> Self {
        Self::Io(err)
    }
}

impl From<serde_json::Error> for IndexError {
    fn from(err: serde_json::Error) -> Self {
        Self::Json(err)
    }
}

pub fn append_keyframe_index(
    index_dir: &Path,
    entry: &KeyframeIndexEntry,
) -> Result<(), IndexError> {
    append_line(index_dir.join("keyframes.jsonl"), entry)
}

pub fn append_patch_index(index_dir: &Path, entry: &PatchIndexEntry) -> Result<(), IndexError> {
    append_line(index_dir.join("patches.jsonl"), entry)
}

pub fn load_keyframe_index(index_dir: &Path) -> Result<Vec<KeyframeIndexEntry>, IndexError> {
    load_lines(index_dir.join("keyframes.jsonl"))
}

pub fn load_patch_index(index_dir: &Path) -> Result<Vec<PatchIndexEntry>, IndexError> {
    load_lines(index_dir.join("patches.jsonl"))
}

pub fn nearest_keyframe(
    index_dir: &Path,
    timestamp_ms: u64,
) -> Result<Option<KeyframeIndexEntry>, IndexError> {
    let mut entries = load_keyframe_index(index_dir)?;
    entries.retain(|entry| entry.timestamp_ms <= timestamp_ms);
    entries.sort_by_key(|entry| entry.timestamp_ms);
    Ok(entries.pop())
}

pub fn patch_entries_between(
    index_dir: &Path,
    start_timestamp_ms: u64,
    end_timestamp_ms: u64,
) -> Result<Vec<PatchIndexEntry>, IndexError> {
    let mut entries = load_patch_index(index_dir)?;
    entries.retain(|entry| {
        entry.timestamp_ms >= start_timestamp_ms && entry.timestamp_ms <= end_timestamp_ms
    });
    entries.sort_by_key(|entry| (entry.timestamp_ms, entry.sequence));
    Ok(entries)
}

fn append_line<T: Serialize>(path: impl AsRef<Path>, entry: &T) -> Result<(), IndexError> {
    let line = serde_json::to_string(entry)?;
    let mut file = OpenOptions::new().create(true).append(true).open(path)?;
    writeln!(file, "{line}")?;
    Ok(())
}

fn load_lines<T: for<'de> Deserialize<'de>>(path: impl AsRef<Path>) -> Result<Vec<T>, IndexError> {
    let path = path.as_ref();
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
        entries.push(serde_json::from_str::<T>(&line)?);
    }
    Ok(entries)
}
