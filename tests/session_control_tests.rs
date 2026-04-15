use screen_timeline_recorder::{
    config::RecorderConfig,
    recorder::RecordingStats,
    session::SessionState,
    session_control::{delete_session, pause_session, read_status, resume_session, stop_session},
    storage::{SessionDimensions, Storage},
};

fn create_session(output_dir: &std::path::Path, session_id: &str) -> Storage {
    Storage::start_session(
        RecorderConfig::default().with_output_dir(output_dir.to_path_buf()),
        session_id,
        1_000,
        SessionDimensions {
            display_width: 4,
            display_height: 4,
            working_width: 4,
            working_height: 4,
        },
    )
    .expect("start session")
}

#[test]
fn pause_resume_and_stop_operate_on_signal_files() {
    let temp_dir = tempfile::tempdir().expect("tempdir");
    let storage = create_session(temp_dir.path(), "session-alpha");
    let root = storage.layout().root().to_path_buf();

    pause_session(temp_dir.path(), "session-alpha").expect("pause session");
    assert!(root.join("pause.signal").exists());

    resume_session(temp_dir.path(), "session-alpha").expect("resume session");
    assert!(!root.join("pause.signal").exists());

    stop_session(temp_dir.path(), "session-alpha").expect("stop session");
    assert!(root.join("stop.signal").exists());
}

#[test]
fn read_status_returns_existing_status_json() {
    let temp_dir = tempfile::tempdir().expect("tempdir");
    let mut storage = create_session(temp_dir.path(), "session-alpha");
    let stats = RecordingStats {
        frames_seen: 3,
        identical_frames_skipped: 1,
        sampled_precheck_skipped: 0,
        diff_runs: 2,
        patch_frames_written: 1,
        patch_regions_written: 2,
        keyframes_written: 1,
        started_at: 1_000,
        finished_at: 2_000,
    };
    storage
        .write_status(SessionState::Paused, &stats)
        .expect("write status");

    let status = read_status(temp_dir.path(), "session-alpha").expect("read status");

    assert_eq!(status.session_id, "session-alpha");
    assert_eq!(status.state, SessionState::Paused);
    assert_eq!(status.stats, stats);
}

#[test]
fn control_commands_fail_for_missing_session() {
    let temp_dir = tempfile::tempdir().expect("tempdir");

    let error = pause_session(temp_dir.path(), "missing-session").expect_err("missing session");

    assert!(error.to_string().contains("session not found"));
}

#[test]
fn delete_session_removes_stopped_session_directory() {
    let temp_dir = tempfile::tempdir().expect("tempdir");
    let mut storage = create_session(temp_dir.path(), "session-delete");
    storage
        .write_status(SessionState::Stopped, &RecordingStats::default())
        .expect("write status");
    let root = storage.layout().root().to_path_buf();

    delete_session(temp_dir.path(), "session-delete").expect("delete session");

    assert!(!root.exists());
}

#[test]
fn delete_session_rejects_active_session() {
    let temp_dir = tempfile::tempdir().expect("tempdir");
    create_session(temp_dir.path(), "session-active");

    let error = delete_session(temp_dir.path(), "session-active")
        .expect_err("active session should not be deleted");

    assert!(
        error
            .to_string()
            .contains("cannot delete an active session")
    );
}
