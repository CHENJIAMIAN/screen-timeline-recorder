use screen_timeline_recorder::{
    config::RecorderConfig,
    retention::{enforce_max_sessions, enforce_retention},
    storage::{SessionDimensions, Storage},
};

fn create_session(output_dir: &std::path::Path, session_id: &str, started_at: u64) {
    let mut storage = Storage::start_session(
        RecorderConfig::default().with_output_dir(output_dir.to_path_buf()),
        session_id,
        started_at,
        SessionDimensions {
            display_width: 4,
            display_height: 4,
            working_width: 4,
            working_height: 4,
        },
    )
    .expect("start session");
    storage
        .finalize_session(started_at + 1_000)
        .expect("finalize session");
}

#[test]
fn retention_noops_when_limit_is_unset() {
    let temp_dir = tempfile::tempdir().expect("tempdir");
    create_session(temp_dir.path(), "session-alpha", 1_000);
    create_session(temp_dir.path(), "session-beta", 2_000);

    let report = enforce_max_sessions(temp_dir.path(), None).expect("retention report");

    assert!(report.removed_sessions.is_empty());
    assert!(temp_dir.path().join("sessions/session-alpha").exists());
    assert!(temp_dir.path().join("sessions/session-beta").exists());
}

#[test]
fn retention_keeps_only_newest_sessions() {
    let temp_dir = tempfile::tempdir().expect("tempdir");
    create_session(temp_dir.path(), "session-alpha", 1_000);
    create_session(temp_dir.path(), "session-beta", 2_000);
    create_session(temp_dir.path(), "session-gamma", 3_000);

    let report = enforce_max_sessions(temp_dir.path(), Some(2)).expect("retention report");

    assert_eq!(report.removed_sessions, vec!["session-alpha".to_string()]);
    assert!(!temp_dir.path().join("sessions/session-alpha").exists());
    assert!(temp_dir.path().join("sessions/session-beta").exists());
    assert!(temp_dir.path().join("sessions/session-gamma").exists());
}

#[test]
fn retention_ignores_missing_sessions_root() {
    let temp_dir = tempfile::tempdir().expect("tempdir");

    let report = enforce_max_sessions(temp_dir.path(), Some(2)).expect("retention report");

    assert!(report.removed_sessions.is_empty());
}

#[test]
fn retention_removes_sessions_older_than_max_age_days() {
    let temp_dir = tempfile::tempdir().expect("tempdir");
    let day_ms = 24_u64 * 60 * 60 * 1_000;
    let now = 30 * day_ms;
    create_session(temp_dir.path(), "session-old", now - (10 * day_ms));
    create_session(temp_dir.path(), "session-new", now - (2 * day_ms));

    let report =
        enforce_retention(temp_dir.path(), None, Some(5), None, now).expect("retention report");

    assert_eq!(report.removed_sessions, vec!["session-old".to_string()]);
    assert!(!temp_dir.path().join("sessions/session-old").exists());
    assert!(temp_dir.path().join("sessions/session-new").exists());
}

#[test]
fn retention_applies_age_and_count_constraints_together() {
    let temp_dir = tempfile::tempdir().expect("tempdir");
    let day_ms = 24_u64 * 60 * 60 * 1_000;
    let now = 50 * day_ms;
    create_session(temp_dir.path(), "session-ancient", now - (20 * day_ms));
    create_session(temp_dir.path(), "session-old", now - (8 * day_ms));
    create_session(temp_dir.path(), "session-recent", now - (3 * day_ms));
    create_session(temp_dir.path(), "session-current", now - day_ms);

    let report =
        enforce_retention(temp_dir.path(), Some(2), Some(7), None, now).expect("retention report");

    assert_eq!(
        report.removed_sessions,
        vec!["session-old".to_string(), "session-ancient".to_string()]
    );
    assert!(!temp_dir.path().join("sessions/session-ancient").exists());
    assert!(!temp_dir.path().join("sessions/session-old").exists());
    assert!(temp_dir.path().join("sessions/session-recent").exists());
    assert!(temp_dir.path().join("sessions/session-current").exists());
}

#[test]
fn retention_removes_oldest_sessions_until_total_bytes_fit_limit() {
    let temp_dir = tempfile::tempdir().expect("tempdir");
    create_session(temp_dir.path(), "session-alpha", 1_000);
    create_session(temp_dir.path(), "session-beta", 2_000);
    create_session(temp_dir.path(), "session-gamma", 3_000);

    std::fs::write(
        temp_dir
            .path()
            .join("sessions/session-alpha/filler-alpha.bin"),
        vec![1_u8; 256],
    )
    .expect("alpha filler");
    std::fs::write(
        temp_dir
            .path()
            .join("sessions/session-beta/filler-beta.bin"),
        vec![2_u8; 256],
    )
    .expect("beta filler");
    std::fs::write(
        temp_dir
            .path()
            .join("sessions/session-gamma/filler-gamma.bin"),
        vec![3_u8; 256],
    )
    .expect("gamma filler");

    let report = enforce_retention(temp_dir.path(), None, None, Some(700), u64::MAX)
        .expect("retention report");

    assert_eq!(
        report.removed_sessions,
        vec!["session-beta".to_string(), "session-alpha".to_string()]
    );
    assert!(!temp_dir.path().join("sessions/session-alpha").exists());
    assert!(!temp_dir.path().join("sessions/session-beta").exists());
    assert!(temp_dir.path().join("sessions/session-gamma").exists());
}
