use screen_timeline_recorder::viewer_server::{ContentType, ViewerServer};
use serde_json::Value;

fn make_server() -> ViewerServer {
    ViewerServer::new("D:/Desktop/screen-timeline-recorder", "2026-04-13")
}

#[test]
fn serves_index_html_for_root() {
    let server = make_server();
    let response = server.handle_get("/").expect("root response");

    assert_eq!(response.status_code, 200);
    assert_eq!(response.content_type, ContentType::Html);
    let body = String::from_utf8(response.body).expect("html");
    assert!(body.contains("Screen Timeline Viewer"));
    assert!(body.contains("id=\"app\""));
    assert!(body.contains("<script type=\"module\" src=\"app.js\"></script>"));
    assert!(!body.contains("id=\"timeline\""));
    assert!(!body.contains("id=\"canvas\""));
    assert!(!body.contains("id=\"control-pause\""));
    assert!(!body.contains("id=\"control-resume\""));
}

#[test]
fn serves_static_assets_by_name() {
    let server = make_server();
    let response = server.handle_get("/app.js").expect("app.js response");

    assert_eq!(response.status_code, 200);
    assert_eq!(response.content_type, ContentType::JavaScript);
    let body = String::from_utf8(response.body).expect("js");
    assert!(body.contains("vue.esm-browser.prod.js"));
    assert!(body.contains("viewer_app.js"));
}

#[test]
fn serves_vue_viewer_module_asset() {
    let server = make_server();
    let response = server
        .handle_get("/viewer_app.js")
        .expect("viewer_app.js response");

    assert_eq!(response.status_code, 200);
    assert_eq!(response.content_type, ContentType::JavaScript);
    let body = String::from_utf8(response.body).expect("js");
    assert!(body.contains("loadSession"));
    assert!(body.contains("fetchControl"));
    assert!(body.contains("fetchJson"));
    assert!(body.contains("loadVideoSegments"));
    assert!(body.contains("applyPlaybackPreferences"));
}

#[test]
fn serves_vue_api_client_asset() {
    let server = make_server();
    let response = server
        .handle_get("/api_client.js")
        .expect("api_client.js response");

    assert_eq!(response.status_code, 200);
    assert_eq!(response.content_type, ContentType::JavaScript);
    let body = String::from_utf8(response.body).expect("js");
    assert!(body.contains("/api/control"));
    assert!(body.contains("/api/autostart/save"));
    assert!(body.contains("/api/recording-settings/save"));
    assert!(!body.contains("/api/frame"));
    assert!(!body.contains("/api/patches"));
}

#[test]
fn serves_video_segment_metadata_json() {
    let temp_dir = tempfile::tempdir().expect("tempdir");
    let server = ViewerServer::new(temp_dir.path(), "video-session");
    let session_root = temp_dir.path().join("sessions").join("video-session");
    std::fs::create_dir_all(session_root.join("index")).expect("index dir");
    std::fs::create_dir_all(session_root.join("segments")).expect("segments dir");
    std::fs::write(
        session_root.join("manifest.json"),
        r#"{
  "session_id": "video-session",
  "started_at": 1000,
  "finished_at": 4500,
  "display_width": 1920,
  "display_height": 1080,
  "video_width": 1440,
  "video_height": 810,
  "recording_format": "video-segments",
  "segment_duration_ms": 30000,
  "video_codec": "h264",
  "recorder_version": "0.1.0",
  "viewer_default_zoom": 1.0,
  "viewer_overlay_enabled_by_default": false,
  "burn_in_enabled": true,
  "viewer_language": "auto"
}"#,
    )
    .expect("manifest");
    std::fs::write(
        session_root.join("index").join("segments.jsonl"),
        "{\"sequence\":0,\"started_at\":1000,\"finished_at\":4500,\"relative_path\":\"segments/000000.mp4\",\"bytes\":71505}\n",
    )
    .expect("index");

    let response = server
        .handle_get("/api/segments")
        .expect("segments response");

    assert_eq!(response.status_code, 200);
    assert_eq!(response.content_type, ContentType::Json);
    let body = String::from_utf8(response.body).expect("json");
    assert!(body.contains("\"relative_path\":\"segments/000000.mp4\""));
    assert!(body.contains("\"bytes\":71505"));
}

#[test]
fn serves_segment_mp4_assets_from_session_root() {
    let temp_dir = tempfile::tempdir().expect("tempdir");
    let server = ViewerServer::new(temp_dir.path(), "video-session");
    let session_root = temp_dir.path().join("sessions").join("video-session");
    std::fs::create_dir_all(session_root.join("segments")).expect("segments dir");
    std::fs::write(session_root.join("segments").join("000000.mp4"), b"fake-mp4").expect("mp4");

    let response = server
        .handle_get("/segments/000000.mp4")
        .expect("segment response");

    assert_eq!(response.status_code, 200);
    assert_eq!(response.content_type, ContentType::Mp4);
    assert_eq!(response.body, b"fake-mp4");
}

#[test]
fn segment_assets_honor_session_query_parameter() {
    let temp_dir = tempfile::tempdir().expect("tempdir");
    let server = ViewerServer::new(temp_dir.path(), "session-alpha");
    let sessions_root = temp_dir.path().join("sessions");
    let alpha_root = sessions_root.join("session-alpha").join("segments");
    let beta_root = sessions_root.join("session-beta").join("segments");
    std::fs::create_dir_all(&alpha_root).expect("alpha segments dir");
    std::fs::create_dir_all(&beta_root).expect("beta segments dir");
    std::fs::write(alpha_root.join("000000.mp4"), b"alpha").expect("alpha mp4");
    std::fs::write(beta_root.join("000000.mp4"), b"beta").expect("beta mp4");

    let response = server
        .handle_get("/segments/000000.mp4?session_id=session-beta")
        .expect("segment response");

    assert_eq!(response.status_code, 200);
    assert_eq!(response.content_type, ContentType::Mp4);
    assert_eq!(response.body, b"beta");
}

#[test]
fn rejects_unknown_paths() {
    let server = make_server();
    let response = server.handle_get("/missing").expect("missing response");

    assert_eq!(response.status_code, 404);
    assert_eq!(response.content_type, ContentType::Text);
}

#[test]
fn legacy_frame_and_patch_endpoints_are_not_exposed() {
    let server = make_server();

    let frame = server.handle_get("/api/frame?ts=123").expect("frame response");
    let patches = server
        .handle_get("/api/patches?ts=123")
        .expect("patch response");

    assert_eq!(frame.status_code, 404);
    assert_eq!(frame.content_type, ContentType::Text);
    assert_eq!(patches.status_code, 404);
    assert_eq!(patches.content_type, ContentType::Text);
}

#[test]
fn serves_status_json() {
    let temp_dir = tempfile::tempdir().expect("tempdir");
    let server = ViewerServer::new(temp_dir.path(), "2026-04-13");
    let session_root = temp_dir.path().join("sessions").join("2026-04-13");
    std::fs::create_dir_all(&session_root).expect("session dir");
    std::fs::write(
        session_root.join("status.json"),
        r#"{
  "session_id": "2026-04-13",
  "state": "running",
  "recording": true,
  "stats": {
    "frames_seen": 12,
    "identical_frames_skipped": 4,
    "sampled_precheck_skipped": 3,
    "diff_runs": 5,
    "patch_frames_written": 2,
    "patch_regions_written": 7,
    "keyframes_written": 1,
    "started_at": 1000,
    "finished_at": 5000
  }
}"#,
    )
    .expect("write status");

    let response = server.handle_get("/api/status").expect("status response");
    assert_eq!(response.status_code, 200);
    assert_eq!(response.content_type, ContentType::Json);
    assert!(
        String::from_utf8(response.body)
            .expect("json")
            .contains("\"recording\":true")
    );
}

#[test]
fn serves_sessions_json() {
    let temp_dir = tempfile::tempdir().expect("tempdir");
    let server = ViewerServer::new(temp_dir.path(), "2026-04-13");
    let sessions_root = temp_dir.path().join("sessions");
    let alpha_root = sessions_root.join("session-alpha");
    let beta_root = sessions_root.join("session-beta");
    std::fs::create_dir_all(&alpha_root).expect("alpha dir");
    std::fs::create_dir_all(&beta_root).expect("beta dir");
    std::fs::write(
        alpha_root.join("manifest.json"),
        r#"{
  "session_id": "session-alpha",
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
}"#,
    )
    .expect("alpha manifest");
    std::fs::write(
        beta_root.join("manifest.json"),
        r#"{
  "session_id": "session-beta",
  "started_at": 3000,
  "finished_at": 4000,
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
}"#,
    )
    .expect("beta manifest");
    std::fs::write(
        beta_root.join("status.json"),
        r#"{
  "session_id": "session-beta",
  "state": "paused",
  "recording": true,
  "stats": {
    "frames_seen": 8,
    "identical_frames_skipped": 2,
    "sampled_precheck_skipped": 1,
    "diff_runs": 5,
    "patch_frames_written": 2,
    "patch_regions_written": 3,
    "keyframes_written": 1,
    "started_at": 3000,
    "finished_at": 4000
  }
}"#,
    )
    .expect("beta status");

    let response = server
        .handle_get("/api/sessions")
        .expect("sessions response");

    assert_eq!(response.status_code, 200);
    assert_eq!(response.content_type, ContentType::Json);
    let body = String::from_utf8(response.body).expect("json");
    assert!(body.contains("\"session-beta\""));
    assert!(body.contains("\"session-alpha\""));
    assert!(body.contains("\"state\":\"paused\""));
    assert!(body.contains("\"total_bytes\":"));
}

#[test]
fn serves_autostart_status_json() {
    let temp_dir = tempfile::tempdir().expect("tempdir");
    let server = ViewerServer::new(temp_dir.path(), "2026-04-13");
    let response = server
        .handle_get("/api/autostart")
        .expect("autostart response");

    assert_eq!(response.status_code, 200);
    assert_eq!(response.content_type, ContentType::Json);
    let body = String::from_utf8(response.body).expect("json");
    assert!(body.contains("\"task_name\":\"ScreenTimelineRecorder_Autostart\""));
    assert!(body.contains("\"settings\""));
}

#[test]
fn serves_recording_settings_json() {
    let temp_dir = tempfile::tempdir().expect("tempdir");
    let server = ViewerServer::new(temp_dir.path(), "2026-04-13");
    let response = server
        .handle_get("/api/recording-settings")
        .expect("recording settings response");

    assert_eq!(response.status_code, 200);
    assert_eq!(response.content_type, ContentType::Json);
    let body = String::from_utf8(response.body).expect("json");
    assert!(body.contains("\"sampling_interval_ms\":100"));
    assert!(body.contains("\"working_scale\":1.0"));
    assert!(body.contains("\"burn_in_enabled\":true"));
}

#[test]
fn recording_settings_save_updates_supported_fields() {
    let temp_dir = tempfile::tempdir().expect("tempdir");
    let server = ViewerServer::new(temp_dir.path(), "2026-04-13");
    let response = server
        .handle_get(
            "/api/recording-settings/save?sampling_interval_ms=750&working_scale=0.6&burn_in_enabled=0",
        )
        .expect("recording settings save response");

    assert_eq!(response.status_code, 200);
    assert_eq!(response.content_type, ContentType::Json);
    let body: Value = serde_json::from_slice(&response.body).expect("parse json");
    assert_eq!(body["sampling_interval_ms"].as_u64(), Some(750));
    assert_eq!(body["working_scale"].as_f64(), Some(0.6));
    assert_eq!(body["burn_in_enabled"].as_bool(), Some(false));
}

#[test]
fn control_invalid_action_returns_error() {
    let temp_dir = tempfile::tempdir().expect("tempdir");
    let server = ViewerServer::new(temp_dir.path(), "2026-04-13");
    let response = server
        .handle_get("/api/control?action=fast_forward")
        .expect("control response");

    assert_eq!(response.status_code, 400);
    assert_eq!(response.content_type, ContentType::Json);
    let body: Value = serde_json::from_slice(&response.body).expect("parse json");
    assert_eq!(body["ok"], Value::Bool(false));
    assert_eq!(body["action"].as_str(), Some("fast_forward"));
    assert_eq!(
        body["error"].as_str(),
        Some("missing or invalid control action")
    );
}

#[test]
fn control_stop_creates_stop_signal() {
    let temp_dir = tempfile::tempdir().expect("tempdir");
    let server = ViewerServer::new(temp_dir.path(), "2026-04-13");
    let session_root = temp_dir.path().join("sessions").join("2026-04-13");
    std::fs::create_dir_all(&session_root).expect("session dir");
    std::fs::write(
        session_root.join("status.json"),
        r#"{
  "session_id": "2026-04-13",
  "state": "running",
  "recording": true,
  "stats": {
    "frames_seen": 1,
    "identical_frames_skipped": 0,
    "sampled_precheck_skipped": 0,
    "diff_runs": 1,
    "patch_frames_written": 0,
    "patch_regions_written": 0,
    "keyframes_written": 1,
    "started_at": 1000,
    "finished_at": 1000
  }
}"#,
    )
    .expect("write status");

    let stop_response = server
        .handle_get("/api/control?action=stop")
        .expect("stop response");

    assert_eq!(stop_response.status_code, 200);
    assert_eq!(stop_response.content_type, ContentType::Json);
    let body: Value = serde_json::from_slice(&stop_response.body).expect("parse json");
    assert_eq!(body["ok"], Value::Bool(true));
    assert_eq!(body["action"].as_str(), Some("stop"));
    assert_eq!(body["session_id"].as_str(), Some("2026-04-13"));
    assert!(session_root.join("stop.signal").exists());
}

#[test]
fn control_start_returns_error_when_ffmpeg_is_missing() {
    let temp_dir = tempfile::tempdir().expect("tempdir");
    let server = ViewerServer::new(temp_dir.path(), "2026-04-13");

    let response = server
        .handle_get("/api/control?action=start")
        .expect("start response");

    assert_eq!(response.status_code, 409);
    assert_eq!(response.content_type, ContentType::Json);
    let body: Value = serde_json::from_slice(&response.body).expect("parse json");
    assert_eq!(body["ok"], Value::Bool(false));
    assert_eq!(body["action"].as_str(), Some("start"));
    assert!(body["error"]
        .as_str()
        .expect("error message")
        .contains("ffmpeg sidecar not found"));
}

#[test]
fn session_query_parameter_overrides_default_session() {
    let temp_dir = tempfile::tempdir().expect("tempdir");
    let server = ViewerServer::new(temp_dir.path(), "session-alpha");
    let sessions_root = temp_dir.path().join("sessions");
    let alpha_root = sessions_root.join("session-alpha");
    let beta_root = sessions_root.join("session-beta");
    std::fs::create_dir_all(&alpha_root).expect("alpha dir");
    std::fs::create_dir_all(&beta_root).expect("beta dir");
    std::fs::write(
        alpha_root.join("manifest.json"),
        r#"{
  "session_id": "session-alpha",
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
}"#,
    )
    .expect("alpha manifest");
    std::fs::write(
        beta_root.join("manifest.json"),
        r#"{
  "session_id": "session-beta",
  "started_at": 3000,
  "finished_at": 4000,
  "display_width": 1280,
  "display_height": 720,
  "video_width": 640,
  "video_height": 360,
  "recording_format": "video-segments",
  "segment_duration_ms": 30000,
  "video_codec": "h264",
  "recorder_version": "0.1.0",
  "viewer_default_zoom": 1.0,
  "viewer_overlay_enabled_by_default": false,
  "burn_in_enabled": true,
  "viewer_language": "auto"
}"#,
    )
    .expect("beta manifest");

    let response = server
        .handle_get("/api/session?session_id=session-beta")
        .expect("session response");

    assert_eq!(response.status_code, 200);
    let body = String::from_utf8(response.body).expect("json");
    assert!(body.contains("\"session_id\":\"session-beta\""));
    assert!(body.contains("\"working_width\":640"));
}

#[test]
fn stale_session_query_falls_back_to_default_session() {
    let temp_dir = tempfile::tempdir().expect("tempdir");
    let server = ViewerServer::new(temp_dir.path(), "session-alpha");
    let alpha_root = temp_dir.path().join("sessions").join("session-alpha");
    std::fs::create_dir_all(&alpha_root).expect("alpha dir");
    std::fs::write(
        alpha_root.join("manifest.json"),
        r#"{
  "session_id": "session-alpha",
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
}"#,
    )
    .expect("alpha manifest");

    let response = server
        .handle_get("/api/session?session_id=session-missing")
        .expect("session response");

    assert_eq!(response.status_code, 200);
    let body = String::from_utf8(response.body).expect("json");
    assert!(body.contains("\"session_id\":\"session-alpha\""));
}
