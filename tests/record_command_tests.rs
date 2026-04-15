use std::cell::Cell;

use screen_timeline_recorder::{
    config::RecorderConfig,
    recorder::{
        RecordingStats, pause_signal_requested, record_command_with_stop, stop_signal_requested,
    },
};

#[cfg(target_os = "windows")]
#[test]
fn record_command_captures_a_real_session() {
    let temp_dir = tempfile::tempdir().expect("tempdir");
    let config = RecorderConfig::default().with_output_dir(temp_dir.path().to_path_buf());
    let stop_calls = Cell::new(0usize);
    let storage = record_command_with_stop(config, "test-session", || {
        let next = stop_calls.get() + 1;
        stop_calls.set(next);
        next > 1
    })
    .expect("expected record command to work");

    assert!(storage.layout().manifest_path().exists());
    let manifest = storage.load_manifest().expect("manifest");
    assert_eq!(manifest.session_id, "test-session");
    assert!(manifest.finished_at.is_some());
    assert!(manifest.display_width > 0);
    assert!(manifest.display_height > 0);
}

#[cfg(not(target_os = "windows"))]
#[test]
fn record_command_reports_unsupported_platform() {
    let temp_dir = tempfile::tempdir().expect("tempdir");
    let config = RecorderConfig::default().with_output_dir(temp_dir.path().to_path_buf());

    let error = record_command(config, "test-session")
        .expect_err("expected record command to fail on non-windows");

    assert!(
        error
            .to_string()
            .contains("recording is only supported on Windows")
    );
}

#[test]
fn recording_stats_summary_is_human_readable() {
    let stats = RecordingStats {
        frames_seen: 12,
        identical_frames_skipped: 5,
        sampled_precheck_skipped: 4,
        diff_runs: 3,
        patch_frames_written: 2,
        patch_regions_written: 6,
        keyframes_written: 2,
        started_at: 1_000,
        finished_at: 4_500,
    };

    assert_eq!(
        stats.summary_line(),
        "frames=12 duration_ms=3500 identical_skips=5 sampled_skips=4 diff_runs=3 patch_frames=2 patch_regions=6 keyframes=2"
    );
}

#[test]
fn stop_signal_requested_detects_existing_file() {
    let temp_dir = tempfile::tempdir().expect("tempdir");
    let signal_path = temp_dir.path().join("stop.signal");
    assert!(!stop_signal_requested(&signal_path));

    std::fs::write(&signal_path, b"stop").expect("write stop signal");

    assert!(stop_signal_requested(&signal_path));
}

#[test]
fn pause_signal_requested_detects_existing_file() {
    let temp_dir = tempfile::tempdir().expect("tempdir");
    let signal_path = temp_dir.path().join("pause.signal");
    assert!(!pause_signal_requested(&signal_path));

    std::fs::write(&signal_path, b"pause").expect("write pause signal");

    assert!(pause_signal_requested(&signal_path));
}
