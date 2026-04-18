use screen_timeline_recorder::{
    config::RecorderConfig,
    config::ViewerLanguage,
    diff::PatchRegion,
    frame::Frame,
    recorder::RecordingStats,
    session::{RecordingFormat, SessionState},
    storage::{SessionDimensions, Storage},
    viewer_api::{get_activity, get_frame_png, get_patches, get_session, get_sessions, get_status},
};

fn start_storage(output_dir: &std::path::Path) -> Storage {
    let config = RecorderConfig::default().with_output_dir(output_dir.to_path_buf());
    Storage::start_session(
        config,
        "2026-04-13",
        1_700_000_000_000,
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
fn session_payload_matches_manifest() {
    let temp_dir = tempfile::tempdir().expect("tempdir");
    let storage = start_storage(temp_dir.path());

    let session = get_session(temp_dir.path(), "2026-04-13").expect("session");

    assert_eq!(session.session_id, storage.manifest().session_id);
    assert_eq!(session.started_at, storage.manifest().started_at);
    assert_eq!(session.finished_at, storage.manifest().finished_at);
    assert_eq!(session.recording_format, RecordingFormat::PatchFrames);
    assert_eq!(session.display_width, storage.manifest().display_width);
    assert_eq!(session.display_height, storage.manifest().display_height);
    assert_eq!(session.working_width, storage.manifest().working_width);
    assert_eq!(session.working_height, storage.manifest().working_height);
    assert_eq!(
        session.sampling_interval_ms,
        storage.manifest().sampling_interval_ms
    );
    assert_eq!(session.block_width, storage.manifest().block_width);
    assert_eq!(session.block_height, storage.manifest().block_height);
    assert_eq!(
        session.keyframe_interval_ms,
        storage.manifest().keyframe_interval_ms
    );
    assert_eq!(
        session.sensitivity_mode,
        storage.manifest().sensitivity_mode
    );
    assert_eq!(
        session.precheck_threshold,
        storage.manifest().precheck_threshold
    );
    assert_eq!(
        session.block_difference_threshold,
        storage.manifest().block_difference_threshold
    );
    assert_eq!(
        session.changed_pixel_ratio_threshold,
        storage.manifest().changed_pixel_ratio_threshold
    );
    assert_eq!(
        session.stability_window,
        storage.manifest().stability_window
    );
    assert_eq!(
        session.compression_format,
        storage.manifest().compression_format
    );
    assert_eq!(
        session.recorder_version,
        storage.manifest().recorder_version
    );
    assert_eq!(
        session.viewer_default_zoom,
        storage.manifest().viewer_default_zoom
    );
    assert_eq!(
        session.viewer_overlay_enabled_by_default,
        storage.manifest().viewer_overlay_enabled_by_default
    );
    assert_eq!(session.burn_in_enabled, storage.manifest().burn_in_enabled);
    assert_eq!(session.viewer_language, ViewerLanguage::Auto);
}

#[test]
fn frame_endpoint_returns_png_bytes() {
    let temp_dir = tempfile::tempdir().expect("tempdir");
    let mut storage = start_storage(temp_dir.path());
    let frame = Frame::solid_rgba(4, 4, [10, 20, 30, 255]);
    storage
        .write_keyframe(100, frame.as_rgba())
        .expect("write keyframe");

    let png_bytes = get_frame_png(temp_dir.path(), "2026-04-13", 100).expect("png");

    let decoder = png::Decoder::new(std::io::Cursor::new(png_bytes));
    let mut reader = decoder.read_info().expect("read info");
    let mut buffer = vec![0u8; reader.output_buffer_size()];
    let info = reader.next_frame(&mut buffer).expect("decode");

    assert_eq!(info.width, 4);
    assert_eq!(info.height, 4);
    assert_eq!(info.color_type, png::ColorType::Rgba);
    assert_eq!(info.bit_depth, png::BitDepth::Eight);

    let pixel = &buffer[0..4];
    assert_eq!(pixel, &[10, 20, 30, 255]);
}

#[test]
fn patches_endpoint_returns_overlay_metadata() {
    let temp_dir = tempfile::tempdir().expect("tempdir");
    let mut storage = start_storage(temp_dir.path());
    let base = Frame::solid_rgba(4, 4, [0, 0, 0, 255]);
    storage
        .write_keyframe(100, base.as_rgba())
        .expect("write keyframe");

    storage
        .write_patches(
            120,
            &[
                PatchRegion {
                    x: 0,
                    y: 0,
                    width: 2,
                    height: 1,
                    data: vec![255, 0, 0, 255].repeat(2),
                },
                PatchRegion {
                    x: 2,
                    y: 2,
                    width: 2,
                    height: 2,
                    data: vec![0, 255, 0, 255].repeat(4),
                },
            ],
        )
        .expect("write patches");

    let patches = get_patches(temp_dir.path(), "2026-04-13", 120).expect("patches");

    assert_eq!(patches.len(), 2);
    assert_eq!(patches[0].timestamp_ms, 120);
    assert_eq!(patches[0].sequence, 0);
    assert_eq!(patches[0].x, 0);
    assert_eq!(patches[0].y, 0);
    assert_eq!(patches[0].width, 2);
    assert_eq!(patches[0].height, 1);

    assert_eq!(patches[1].timestamp_ms, 120);
    assert_eq!(patches[1].sequence, 1);
    assert_eq!(patches[1].x, 2);
    assert_eq!(patches[1].y, 2);
    assert_eq!(patches[1].width, 2);
    assert_eq!(patches[1].height, 2);
}

#[test]
fn status_endpoint_returns_live_session_stats() {
    let temp_dir = tempfile::tempdir().expect("tempdir");
    let mut storage = start_storage(temp_dir.path());
    let stats = RecordingStats {
        frames_seen: 8,
        identical_frames_skipped: 2,
        sampled_precheck_skipped: 3,
        diff_runs: 3,
        patch_frames_written: 2,
        patch_regions_written: 4,
        keyframes_written: 1,
        started_at: 1_000,
        finished_at: 4_000,
    };
    storage
        .write_status(SessionState::Running, &stats)
        .expect("write status");

    let status = get_status(temp_dir.path(), "2026-04-13").expect("get status");

    assert_eq!(status.state, SessionState::Running);
    assert!(status.recording);
    assert_eq!(status.stats, stats);
}

#[test]
fn sessions_endpoint_lists_sessions_sorted_by_recency() {
    let temp_dir = tempfile::tempdir().expect("tempdir");
    let mut alpha = Storage::start_session(
        RecorderConfig::default().with_output_dir(temp_dir.path().to_path_buf()),
        "session-alpha",
        1_000,
        SessionDimensions {
            display_width: 4,
            display_height: 4,
            working_width: 4,
            working_height: 4,
        },
    )
    .expect("alpha");
    alpha.finalize_session(6_000).expect("finalize alpha");

    let mut beta = Storage::start_session(
        RecorderConfig::default().with_output_dir(temp_dir.path().to_path_buf()),
        "session-beta",
        3_000,
        SessionDimensions {
            display_width: 8,
            display_height: 8,
            working_width: 4,
            working_height: 4,
        },
    )
    .expect("beta");
    beta.finalize_session(4_000).expect("finalize beta");

    let sessions = get_sessions(temp_dir.path()).expect("get sessions");

    assert_eq!(sessions.len(), 2);
    assert_eq!(sessions[0].session_id, "session-alpha");
    assert_eq!(sessions[0].last_activity_at, 6_000);
    assert_eq!(sessions[0].recording_format, RecordingFormat::PatchFrames);
    assert!(sessions[0].total_bytes > 0);
    assert_eq!(sessions[1].session_id, "session-beta");
    assert_eq!(sessions[1].last_activity_at, 4_000);
    assert_eq!(sessions[1].recording_format, RecordingFormat::PatchFrames);
}

#[test]
fn sessions_endpoint_includes_status_when_available() {
    let temp_dir = tempfile::tempdir().expect("tempdir");
    let mut storage = start_storage(temp_dir.path());
    storage
        .write_status(
            SessionState::Paused,
            &RecordingStats {
                started_at: 1_000,
                finished_at: 2_000,
                ..RecordingStats::default()
            },
        )
        .expect("write status");

    let sessions = get_sessions(temp_dir.path()).expect("get sessions");
    assert_eq!(sessions.len(), 1);

    let status = sessions[0].status.as_ref().expect("status info");
    assert_eq!(status.state, SessionState::Paused);
    assert!(status.recording);
    assert!(sessions[0].total_bytes > 0);
}

#[test]
fn sessions_endpoint_handles_missing_status_json() {
    let temp_dir = tempfile::tempdir().expect("tempdir");
    start_storage(temp_dir.path());
    let status_path = temp_dir
        .path()
        .join("sessions")
        .join("2026-04-13")
        .join("status.json");
    std::fs::remove_file(status_path).expect("remove status json");

    let sessions = get_sessions(temp_dir.path()).expect("get sessions");
    assert_eq!(sessions.len(), 1);
    assert!(sessions[0].status.is_none());
}

#[test]
fn status_endpoint_returns_running_when_started_equals_finished() {
    let temp_dir = tempfile::tempdir().expect("tempdir");
    let mut storage = start_storage(temp_dir.path());
    storage
        .write_status(
            SessionState::Running,
            &RecordingStats {
                started_at: 1_000,
                finished_at: 1_000,
                ..RecordingStats::default()
            },
        )
        .expect("write stale status");

    let status = get_status(temp_dir.path(), "2026-04-13").expect("get status");
    assert_eq!(status.state, SessionState::Running);
    assert!(status.recording);
    assert_eq!(status.stats.finished_at, status.stats.started_at);

    let sessions = get_sessions(temp_dir.path()).expect("get sessions");
    let session_status = sessions[0].status.as_ref().expect("session status");
    assert_eq!(session_status.state, SessionState::Running);
    assert!(session_status.recording);
}

#[test]
fn stop_signal_forces_viewer_status_to_stopped() {
    let temp_dir = tempfile::tempdir().expect("tempdir");
    let mut storage = start_storage(temp_dir.path());
    storage
        .write_status(
            SessionState::Paused,
            &RecordingStats {
                started_at: 1_000,
                finished_at: u64::MAX,
                ..RecordingStats::default()
            },
        )
        .expect("write status");
    std::fs::write(storage.layout().stop_signal_path(), b"stop").expect("write stop signal");

    let status = get_status(temp_dir.path(), "2026-04-13").expect("get status");
    assert_eq!(status.state, SessionState::Stopped);
    assert!(!status.recording);
}

#[test]
fn pause_signal_forces_viewer_status_to_paused() {
    let temp_dir = tempfile::tempdir().expect("tempdir");
    let mut storage = start_storage(temp_dir.path());
    storage
        .write_status(
            SessionState::Running,
            &RecordingStats {
                started_at: 1_000,
                finished_at: 5_000,
                ..RecordingStats::default()
            },
        )
        .expect("write status");
    std::fs::write(storage.layout().pause_signal_path(), b"pause").expect("write pause signal");

    let status = get_status(temp_dir.path(), "2026-04-13").expect("get status");
    assert_eq!(status.state, SessionState::Paused);
    assert!(status.recording);

    let sessions = get_sessions(temp_dir.path()).expect("get sessions");
    let session_status = sessions[0].status.as_ref().expect("session status");
    assert_eq!(session_status.state, SessionState::Paused);
    assert!(session_status.recording);
}

#[test]
fn activity_endpoint_groups_patch_counts_by_timestamp() {
    let temp_dir = tempfile::tempdir().expect("tempdir");
    let mut storage = start_storage(temp_dir.path());

    storage
        .write_keyframe(100, Frame::solid_rgba(4, 4, [0, 0, 0, 255]).as_rgba())
        .expect("write keyframe");
    storage
        .write_patches(
            120,
            &[
                PatchRegion {
                    x: 0,
                    y: 0,
                    width: 2,
                    height: 1,
                    data: vec![255, 0, 0, 255].repeat(2),
                },
                PatchRegion {
                    x: 2,
                    y: 2,
                    width: 2,
                    height: 2,
                    data: vec![0, 255, 0, 255].repeat(4),
                },
            ],
        )
        .expect("write patches");
    storage
        .write_patches(
            160,
            &[PatchRegion {
                x: 1,
                y: 1,
                width: 1,
                height: 1,
                data: vec![255, 255, 255, 255],
            }],
        )
        .expect("write patches");

    let activity = get_activity(temp_dir.path(), "2026-04-13").expect("activity");

    assert_eq!(activity.len(), 2);
    assert_eq!(activity[0].timestamp_ms, 120);
    assert_eq!(activity[0].patch_count, 2);
    assert_eq!(activity[1].timestamp_ms, 160);
    assert_eq!(activity[1].patch_count, 1);
}
