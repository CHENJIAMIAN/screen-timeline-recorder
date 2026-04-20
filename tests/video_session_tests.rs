use screen_timeline_recorder::{
    config::ViewerLanguage,
    session::RecordingFormat,
    video_session::{
        VideoSegmentEntry, VideoSessionManifest, append_video_segment_index, load_video_segment_index,
    },
};

#[test]
fn video_session_manifest_round_trips_with_video_segment_format() {
    let manifest = VideoSessionManifest {
        session_id: "session-video".to_string(),
        started_at: 1_700_000_000_000,
        finished_at: Some(1_700_000_030_000),
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
    };

    let json = serde_json::to_string(&manifest).expect("serialize manifest");
    let loaded: VideoSessionManifest = serde_json::from_str(&json).expect("deserialize manifest");

    assert_eq!(loaded.recording_format, RecordingFormat::VideoSegments);
    assert_eq!(loaded.segment_duration_ms, 30_000);
    assert_eq!(loaded.video_codec, "h264");
}

#[test]
fn appends_and_loads_video_segment_index_entries() {
    let temp_dir = tempfile::tempdir().expect("tempdir");
    let index_path = temp_dir.path().join("segments.jsonl");

    append_video_segment_index(
        &index_path,
        &VideoSegmentEntry {
            sequence: 0,
            started_at: 1_000,
            finished_at: Some(31_000),
            relative_path: "segments/000000.mp4".to_string(),
            bytes: 12_345,
        },
    )
    .expect("append first");
    append_video_segment_index(
        &index_path,
        &VideoSegmentEntry {
            sequence: 1,
            started_at: 31_000,
            finished_at: None,
            relative_path: "segments/000001.mp4".to_string(),
            bytes: 67_890,
        },
    )
    .expect("append second");

    let entries = load_video_segment_index(&index_path).expect("load index");

    assert_eq!(entries.len(), 2);
    assert_eq!(entries[0].sequence, 0);
    assert_eq!(entries[0].relative_path, "segments/000000.mp4");
    assert_eq!(entries[1].sequence, 1);
    assert_eq!(entries[1].finished_at, None);
}

#[test]
fn manifest_load_round_trips_video_segment_format() {
    let json = r#"{
        "session_id": "session-video",
        "started_at": 1000,
        "finished_at": 2000,
        "display_width": 1920,
        "display_height": 1080,
        "video_width": 960,
        "video_height": 540,
        "recording_format": "video-segments",
        "segment_duration_ms": 30000,
        "video_codec": "h264",
        "recorder_version": "0.1.0",
        "viewer_default_zoom": 1.0,
        "viewer_overlay_enabled_by_default": false,
        "burn_in_enabled": true,
        "viewer_language": "auto"
    }"#;

    let manifest: VideoSessionManifest =
        serde_json::from_str(json).expect("deserialize legacy manifest");

    assert_eq!(manifest.recording_format, RecordingFormat::VideoSegments);
}
