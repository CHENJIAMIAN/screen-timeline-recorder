use std::fs;
use std::path::Path;

use crate::session::{SessionLayout, SessionStatus};

#[derive(Debug)]
pub enum SessionControlError {
    MissingSession(String),
    ActiveSession(String),
    Io(std::io::Error),
    Json(serde_json::Error),
}

impl std::fmt::Display for SessionControlError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::MissingSession(session_id) => write!(f, "session not found: {session_id}"),
            Self::ActiveSession(session_id) => {
                write!(f, "cannot delete an active session: {session_id}")
            }
            Self::Io(err) => write!(f, "session control failed: {err}"),
            Self::Json(err) => write!(f, "failed to serialize session status: {err}"),
        }
    }
}

impl std::error::Error for SessionControlError {}

impl From<std::io::Error> for SessionControlError {
    fn from(err: std::io::Error) -> Self {
        Self::Io(err)
    }
}

impl From<serde_json::Error> for SessionControlError {
    fn from(err: serde_json::Error) -> Self {
        Self::Json(err)
    }
}

pub fn pause_session(output_dir: &Path, session_id: &str) -> Result<(), SessionControlError> {
    let layout = checked_layout(output_dir, session_id)?;
    fs::write(layout.pause_signal_path(), b"pause")?;
    Ok(())
}

pub fn resume_session(output_dir: &Path, session_id: &str) -> Result<(), SessionControlError> {
    let layout = checked_layout(output_dir, session_id)?;
    if layout.pause_signal_path().exists() {
        fs::remove_file(layout.pause_signal_path())?;
    }
    Ok(())
}

pub fn stop_session(output_dir: &Path, session_id: &str) -> Result<(), SessionControlError> {
    let layout = checked_layout(output_dir, session_id)?;
    fs::write(layout.stop_signal_path(), b"stop")?;
    Ok(())
}

pub fn delete_session(output_dir: &Path, session_id: &str) -> Result<(), SessionControlError> {
    let layout = checked_layout(output_dir, session_id)?;
    if let Ok(status) = SessionStatus::load(layout.status_path())
        && status.recording
        && !layout.stop_signal_path().exists()
    {
        return Err(SessionControlError::ActiveSession(session_id.to_string()));
    }

    recycle_session_root(layout.root())?;
    Ok(())
}

pub fn read_status(
    output_dir: &Path,
    session_id: &str,
) -> Result<SessionStatus, SessionControlError> {
    let layout = checked_layout(output_dir, session_id)?;
    Ok(SessionStatus::load(layout.status_path())?)
}

pub fn render_status_json(
    output_dir: &Path,
    session_id: &str,
) -> Result<String, SessionControlError> {
    let status = read_status(output_dir, session_id)?;
    Ok(serde_json::to_string_pretty(&status)?)
}

fn checked_layout(
    output_dir: &Path,
    session_id: &str,
) -> Result<SessionLayout, SessionControlError> {
    let layout = SessionLayout::new(output_dir, session_id);
    if !layout.root().exists() {
        return Err(SessionControlError::MissingSession(session_id.to_string()));
    }
    Ok(layout)
}

fn recycle_session_root(path: &Path) -> Result<(), SessionControlError> {
    #[cfg(test)]
    {
        fs::remove_dir_all(path)?;
        Ok(())
    }

    #[cfg(not(test))]
    {
        trash::delete(path).map_err(|err| {
            SessionControlError::Io(std::io::Error::other(format!(
                "failed to move session to recycle bin: {err}"
            )))
        })
    }
}
