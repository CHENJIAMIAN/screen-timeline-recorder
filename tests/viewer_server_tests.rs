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
    assert!(body.contains("Live Status"));
    assert!(body.contains("id=\"timeline\""));
    assert!(body.contains("id=\"session-list\""));
    assert!(body.contains("id=\"activity-strip\""));
    assert!(body.contains("class=\"playback-controls\""));
    assert!(body.contains("id=\"play-pause\""));
    assert!(body.contains("id=\"playback-speed\""));
    assert!(body.contains("id=\"playback-loop\""));
    assert!(body.contains("id=\"playback-loop-label\""));
    assert!(body.contains("<option value=\"0.25\">0.25x</option>"));
    assert!(body.contains("<option value=\"0.5\">0.5x</option>"));
    assert!(body.contains("<option value=\"0.75\">0.75x</option>"));
    assert!(body.contains("<option value=\"1\" selected>1x</option>"));
    assert!(body.contains("<option value=\"1.25\">1.25x</option>"));
    assert!(body.contains("<option value=\"1.5\">1.5x</option>"));
    assert!(body.contains("<option value=\"2\">2x</option>"));
    assert!(body.contains("<option value=\"3\">3x</option>"));
    assert!(body.contains("<option value=\"4\">4x</option>"));
    assert!(body.contains("<option value=\"8\">8x</option>"));
    assert!(body.contains("id=\"quickstart\""));
    assert!(body.contains("id=\"quickstart-title\""));
    assert!(body.contains("id=\"quickstart-step1-title\""));
    assert!(body.contains("id=\"quickstart-step3-body\""));
    assert!(body.contains("id=\"control-pause\""));
    assert!(body.contains("id=\"control-resume\""));
    assert!(body.contains("id=\"control-stop\""));
    assert!(body.contains("id=\"control-start\""));
    assert!(body.contains("id=\"timestamp-friendly\""));
    assert!(body.contains("id=\"advanced-time-toggle\""));
    assert!(body.contains("id=\"advanced-time-label\""));
    assert!(body.contains("id=\"timestamp-help\""));
    assert!(body.contains("id=\"timestamp\""));
    assert!(body.contains("id=\"timestamp-label\""));
    assert!(body.contains("type=\"text\""));
    assert!(body.contains("readonly"));
    assert!(body.contains("class=\"hidden\""));
    assert!(body.contains("id=\"language-label\""));
    assert!(body.contains("id=\"language-select\""));
    assert!(body.contains("<option value=\"auto\">Auto</option>"));
    assert!(body.contains("<option value=\"en\">English</option>"));
    assert!(body.contains("<option value=\"zh\">中文</option>"));
    assert!(body.contains("<span id=\"timestamp-help\" class=\"field-help\">Elapsed since start plus wall-clock time. Enable advanced input for raw milliseconds.</span>"));
    assert!(body.contains("id=\"session-info\""));
    assert!(body.contains("Loading session..."));
    assert!(body.contains("id=\"autostart-settings\""));
    assert!(body.contains("id=\"autostart-title\""));
    assert!(body.contains("id=\"autostart-enabled\""));
    assert!(body.contains("id=\"autostart-output-dir\""));
    assert!(body.contains("id=\"autostart-save\""));
    assert!(body.contains("id=\"recording-settings\""));
    assert!(body.contains("id=\"recording-settings-title\""));
    assert!(body.contains("id=\"recording-sampling-interval\""));
    assert!(body.contains("id=\"recording-sensitivity-mode\""));
    assert!(body.contains("id=\"recording-working-scale\""));
    assert!(body.contains("id=\"recording-burn-in-enabled\""));
    assert!(body.contains("id=\"recording-save\""));
}

#[test]
fn serves_autostart_overlay_elements() {
    let server = make_server();

    let response = server.handle_get("/").expect("root response");

    assert_eq!(response.status_code, 200);
    assert_eq!(response.content_type, ContentType::Html);
    let body = String::from_utf8(response.body).expect("html");
    assert!(body.contains("id=\"autostart-title\""));
    assert!(body.contains("id=\"autostart-subtitle\""));
    assert!(body.contains("id=\"autostart-feedback\""));
    assert!(body.contains("id=\"autostart-refresh\""));
    assert!(body.contains("id=\"autostart-save\""));
}

#[test]
fn serves_static_assets_by_name() {
    let server = make_server();

    let response = server.handle_get("/app.js").expect("app.js response");

    assert_eq!(response.status_code, 200);
    assert_eq!(response.content_type, ContentType::JavaScript);
    let body = String::from_utf8(response.body).expect("js");
    assert!(body.contains("loadSession"));
    assert!(body.contains("/api/status"));
    assert!(body.contains("/api/sessions"));
    assert!(body.contains("/api/activity"));
    assert!(body.contains("/api/control"));
    assert!(body.contains("syncTimelineControls"));
    assert!(body.contains("togglePlayback"));
}

#[test]
fn rejects_unknown_paths() {
    let server = make_server();

    let response = server.handle_get("/missing").expect("missing response");

    assert_eq!(response.status_code, 404);
    assert_eq!(response.content_type, ContentType::Text);
}

#[test]
fn serves_status_json() {
    let temp_dir = tempfile::tempdir().expect("tempdir");
    let server = ViewerServer::new(temp_dir.path(), "2026-04-13");

    let session_root = temp_dir.path().join("sessions").join("2026-04-13");
    std::fs::create_dir_all(&session_root).expect("create session dir");
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
  "working_width": 960,
  "working_height": 540,
  "sampling_interval_ms": 1000,
  "block_width": 32,
  "block_height": 32,
  "keyframe_interval_ms": 60000,
  "sensitivity_mode": "balanced",
  "precheck_threshold": 0.01,
  "block_difference_threshold": 0.05,
  "changed_pixel_ratio_threshold": 0.1,
  "stability_window": 2,
  "compression_format": "raw",
  "recorder_version": "0.1.0",
  "viewer_default_zoom": 1.0,
  "viewer_overlay_enabled_by_default": true
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
  "working_width": 960,
  "working_height": 540,
  "sampling_interval_ms": 1000,
  "block_width": 32,
  "block_height": 32,
  "keyframe_interval_ms": 60000,
  "sensitivity_mode": "balanced",
  "precheck_threshold": 0.01,
  "block_difference_threshold": 0.05,
  "changed_pixel_ratio_threshold": 0.1,
  "stability_window": 2,
  "compression_format": "raw",
  "recorder_version": "0.1.0",
  "viewer_default_zoom": 1.0,
  "viewer_overlay_enabled_by_default": true
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
    assert!(body.contains("\"sampling_interval_ms\":500"));
    assert!(body.contains("\"sensitivity_mode\":\"balanced\""));
    assert!(body.contains("\"burn_in_enabled\":true"));
}

#[test]
fn recording_settings_save_updates_settings() {
    let temp_dir = tempfile::tempdir().expect("tempdir");
    let server = ViewerServer::new(temp_dir.path(), "2026-04-13");

    let response = server
        .handle_get("/api/recording-settings/save?sampling_interval_ms=750&block_width=40&block_height=45&keyframe_interval_ms=90000&working_scale=0.6&burn_in_enabled=0&sensitivity_mode=detailed")
        .expect("recording settings save response");

    assert_eq!(response.status_code, 200);
    assert_eq!(response.content_type, ContentType::Json);
    let body: Value = serde_json::from_slice(&response.body).expect("parse json");
    assert_eq!(body["sampling_interval_ms"].as_u64(), Some(750));
    assert_eq!(body["block_width"].as_u64(), Some(40));
    assert_eq!(body["block_height"].as_u64(), Some(45));
    assert_eq!(body["keyframe_interval_ms"].as_u64(), Some(90000));
    assert_eq!(body["working_scale"].as_f64(), Some(0.6));
    assert_eq!(body["burn_in_enabled"].as_bool(), Some(false));
    assert_eq!(body["sensitivity_mode"], "detailed");
}

#[test]
fn recording_settings_save_invalid_sensitivity_mode_errors() {
    let temp_dir = tempfile::tempdir().expect("tempdir");
    let server = ViewerServer::new(temp_dir.path(), "2026-04-13");

    let err = server
        .handle_get("/api/recording-settings/save?sensitivity_mode=excessive")
        .expect_err("invalid sensitivity_mode should fail");

    assert_eq!(err, "invalid sensitivity_mode: excessive");
}

#[test]
fn serves_control_status_json() {
    let temp_dir = tempfile::tempdir().expect("tempdir");
    let server = ViewerServer::new(temp_dir.path(), "2026-04-13");

    let session_root = temp_dir.path().join("sessions").join("2026-04-13");
    std::fs::create_dir_all(&session_root).expect("create session dir");
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

    let response = server
        .handle_get("/api/control?action=status")
        .expect("control response");

    assert_eq!(response.status_code, 200);
    assert_eq!(response.content_type, ContentType::Json);
    let body = String::from_utf8(response.body).expect("json");
    assert!(body.contains("\"ok\":true"));
    assert!(body.contains("\"action\":\"status\""));
    assert!(body.contains("\"state\":\"running\""));
}

#[test]
fn pause_and_resume_control_toggle_signal_files() {
    let temp_dir = tempfile::tempdir().expect("tempdir");
    let server = ViewerServer::new(temp_dir.path(), "2026-04-13");

    let session_root = temp_dir.path().join("sessions").join("2026-04-13");
    std::fs::create_dir_all(&session_root).expect("create session dir");
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

    let pause_response = server
        .handle_get("/api/control?action=pause")
        .expect("pause response");
    assert_eq!(pause_response.status_code, 200);
    assert!(session_root.join("pause.signal").exists());

    let resume_response = server
        .handle_get("/api/control?action=resume")
        .expect("resume response");
    assert_eq!(resume_response.status_code, 200);
    assert!(!session_root.join("pause.signal").exists());
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
fn control_missing_action_returns_error() {
    let temp_dir = tempfile::tempdir().expect("tempdir");
    let server = ViewerServer::new(temp_dir.path(), "2026-04-13");

    let response = server.handle_get("/api/control").expect("control response");
    assert_eq!(response.status_code, 400);
    assert_eq!(response.content_type, ContentType::Json);
    let body: Value = serde_json::from_slice(&response.body).expect("parse json");
    assert_eq!(body["ok"], Value::Bool(false));
    assert_eq!(body["action"].as_str(), Some(""));
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
    std::fs::create_dir_all(&session_root).expect("create session dir");
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
fn serves_activity_json() {
    let temp_dir = tempfile::tempdir().expect("tempdir");
    let server = ViewerServer::new(temp_dir.path(), "2026-04-13");

    let session_root = temp_dir.path().join("sessions").join("2026-04-13");
    let index_root = session_root.join("index");
    std::fs::create_dir_all(&index_root).expect("index dir");
    std::fs::write(
        index_root.join("patches.jsonl"),
        "{\"timestamp_ms\":120,\"sequence\":0,\"path\":\"patches/120_0.bin\"}\n{\"timestamp_ms\":120,\"sequence\":1,\"path\":\"patches/120_1.bin\"}\n{\"timestamp_ms\":160,\"sequence\":0,\"path\":\"patches/160_0.bin\"}\n",
    )
    .expect("patch index");

    let response = server
        .handle_get("/api/activity")
        .expect("activity response");

    assert_eq!(response.status_code, 200);
    assert_eq!(response.content_type, ContentType::Json);
    let body = String::from_utf8(response.body).expect("json");
    assert!(body.contains("\"timestamp_ms\":120"));
    assert!(body.contains("\"patch_count\":2"));
    assert!(body.contains("\"timestamp_ms\":160"));
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
  "working_width": 960,
  "working_height": 540,
  "sampling_interval_ms": 1000,
  "block_width": 32,
  "block_height": 32,
  "keyframe_interval_ms": 60000,
  "sensitivity_mode": "balanced",
  "precheck_threshold": 0.01,
  "block_difference_threshold": 0.05,
  "changed_pixel_ratio_threshold": 0.1,
  "stability_window": 2,
  "compression_format": "raw",
  "recorder_version": "0.1.0",
  "viewer_default_zoom": 1.0,
  "viewer_overlay_enabled_by_default": true
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
  "working_width": 640,
  "working_height": 360,
  "sampling_interval_ms": 1000,
  "block_width": 32,
  "block_height": 32,
  "keyframe_interval_ms": 60000,
  "sensitivity_mode": "balanced",
  "precheck_threshold": 0.01,
  "block_difference_threshold": 0.05,
  "changed_pixel_ratio_threshold": 0.1,
  "stability_window": 2,
  "compression_format": "raw",
  "recorder_version": "0.1.0",
  "viewer_default_zoom": 1.0,
  "viewer_overlay_enabled_by_default": true
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
