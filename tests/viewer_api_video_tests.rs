use screen_timeline_recorder::{
    config::ViewerLanguage,
    session::{RecordingFormat, SessionState, SessionStatus},
    viewer_api::{get_session, get_sessions, get_status},
    video_session::VideoSessionManifest,
};

#[test]
fn get_session_reads_video_segment_manifest() {
    let temp_dir = tempfile::tempdir().expect("tempdir");
    let session_root = temp_dir.path().join("sessions").join("video-1");
    std::fs::create_dir_all(&session_root).expect("session dir");
    std::fs::write(
        session_root.join("manifest.json"),
        serde_json::to_string_pretty(&VideoSessionManifest {
            session_id: "video-1".to_string(),
            started_at: 1_000,
            finished_at: Some(4_000),
            display_width: 1920,
            display_height: 1080,
            video_width: 1440,
            video_height: 810,
            recording_format: RecordingFormat::VideoSegments,
            segment_duration_ms: 30_000,
            video_codec: "h264".to_string(),
            recorder_version: "0.1.0".to_string(),
            viewer_default_zoom: 1.0,
            viewer_overlay_enabled_by_default: false,
            burn_in_enabled: true,
            viewer_language: ViewerLanguage::Auto,
        })
        .expect("manifest json"),
    )
    .expect("write manifest");
    std::fs::write(
        session_root.join("status.json"),
        serde_json::to_string_pretty(&SessionStatus {
            session_id: "video-1".to_string(),
            state: SessionState::Stopped,
            recording: false,
            stats: screen_timeline_recorder::recording_stats::RecordingStats {
                started_at: 1_000,
                finished_at: 4_000,
                ..Default::default()
            },
        })
        .expect("status json"),
    )
    .expect("write status");

    let session = get_session(temp_dir.path(), "video-1").expect("session");

    assert_eq!(session.recording_format, RecordingFormat::VideoSegments);
    assert_eq!(session.working_width, 1440);
    assert_eq!(session.working_height, 810);
    assert_eq!(session.compression_format, "h264");
}

#[test]
fn get_sessions_lists_video_segment_sessions() {
    let temp_dir = tempfile::tempdir().expect("tempdir");
    let session_root = temp_dir.path().join("sessions").join("video-1");
    std::fs::create_dir_all(&session_root).expect("session dir");
    std::fs::write(
        session_root.join("manifest.json"),
        serde_json::to_string_pretty(&VideoSessionManifest {
            session_id: "video-1".to_string(),
            started_at: 1_000,
            finished_at: Some(4_000),
            display_width: 1920,
            display_height: 1080,
            video_width: 1440,
            video_height: 810,
            recording_format: RecordingFormat::VideoSegments,
            segment_duration_ms: 30_000,
            video_codec: "h264".to_string(),
            recorder_version: "0.1.0".to_string(),
            viewer_default_zoom: 1.0,
            viewer_overlay_enabled_by_default: false,
            burn_in_enabled: true,
            viewer_language: ViewerLanguage::Auto,
        })
        .expect("manifest json"),
    )
    .expect("write manifest");
    std::fs::write(
        session_root.join("status.json"),
        serde_json::to_string_pretty(&SessionStatus {
            session_id: "video-1".to_string(),
            state: SessionState::Stopped,
            recording: false,
            stats: screen_timeline_recorder::recording_stats::RecordingStats {
                started_at: 1_000,
                finished_at: 4_000,
                ..Default::default()
            },
        })
        .expect("status json"),
    )
    .expect("write status");
    std::fs::write(session_root.join("segments.bin"), b"x").expect("dummy payload");

    let sessions = get_sessions(temp_dir.path()).expect("sessions");
    let status = get_status(temp_dir.path(), "video-1").expect("status");

    assert_eq!(sessions.len(), 1);
    assert_eq!(sessions[0].recording_format, RecordingFormat::VideoSegments);
    assert_eq!(sessions[0].working_width, 1440);
    assert_eq!(status.state, SessionState::Stopped);
}

#[test]
fn get_sessions_infers_finished_time_for_stopped_video_session_without_manifest_finish() {
    let temp_dir = tempfile::tempdir().expect("tempdir");
    let session_root = temp_dir.path().join("sessions").join("video-1");
    let segments_root = session_root.join("segments");
    std::fs::create_dir_all(&segments_root).expect("segments dir");
    std::fs::write(
        session_root.join("manifest.json"),
        serde_json::to_string_pretty(&VideoSessionManifest {
            session_id: "video-1".to_string(),
            started_at: 1_000,
            finished_at: None,
            display_width: 1920,
            display_height: 1080,
            video_width: 1440,
            video_height: 810,
            recording_format: RecordingFormat::VideoSegments,
            segment_duration_ms: 30_000,
            video_codec: "h264".to_string(),
            recorder_version: "0.1.0".to_string(),
            viewer_default_zoom: 1.0,
            viewer_overlay_enabled_by_default: false,
            burn_in_enabled: true,
            viewer_language: ViewerLanguage::Auto,
        })
        .expect("manifest json"),
    )
    .expect("write manifest");
    std::fs::write(
        session_root.join("status.json"),
        serde_json::to_string_pretty(&SessionStatus {
            session_id: "video-1".to_string(),
            state: SessionState::Stopped,
            recording: false,
            stats: screen_timeline_recorder::recording_stats::RecordingStats {
                started_at: 1_000,
                finished_at: 1_000,
                ..Default::default()
            },
        })
        .expect("status json"),
    )
    .expect("write status");
    std::fs::write(segments_root.join("000000.mp4"), b"video-bytes").expect("write segment");

    let sessions = get_sessions(temp_dir.path()).expect("sessions");

    assert_eq!(sessions.len(), 1);
    assert!(sessions[0].finished_at.is_some());
    assert!(sessions[0].finished_at.expect("finished_at") > 1_000);
}
