use screen_timeline_recorder::{
    capture::mock::MockCapture,
    capture::{CaptureDimensions, CapturedFrame},
    config::{RecorderConfig, SensitivityMode},
    frame::Frame,
    index::{load_keyframe_index, load_patch_index},
    recorder::{Recorder, RecordingStats},
    session::{SessionState, SessionStatus},
};

fn config_for(output_dir: &std::path::Path) -> RecorderConfig {
    let mut config = RecorderConfig::default().with_output_dir(output_dir.to_path_buf());
    config.block_width = 4;
    config.block_height = 4;
    config.keyframe_interval_ms = 1_000;
    config.sensitivity_mode = SensitivityMode::Detailed;
    config
}

fn dimensions() -> CaptureDimensions {
    CaptureDimensions {
        display_width: 8,
        display_height: 8,
        working_width: 8,
        working_height: 8,
    }
}

fn solid_frame(color: [u8; 4]) -> Frame {
    Frame::solid_rgba(8, 8, color)
}

fn captured(timestamp_ms: u64, frame: Frame) -> CapturedFrame {
    CapturedFrame {
        timestamp_ms,
        frame,
    }
}

#[test]
fn skips_patch_storage_when_frames_do_not_change() {
    let temp_dir = tempfile::tempdir().expect("tempdir");
    let mut config = config_for(temp_dir.path());
    config.keyframe_interval_ms = 10_000;
    let frame = solid_frame([0, 0, 0, 255]);
    let capture = MockCapture::new(
        dimensions(),
        vec![captured(1_000, frame.clone()), captured(2_000, frame)],
    );

    let recorder = Recorder::new(config, "session-unchanged", capture);
    let storage = recorder.run().expect("run recorder");

    let patches = load_patch_index(storage.layout().index_dir()).expect("load patch index");
    assert!(patches.is_empty());

    let keyframes = load_keyframe_index(storage.layout().index_dir()).expect("load keyframe index");
    assert_eq!(keyframes.len(), 1);
}

#[test]
fn writes_patches_when_frames_change() {
    let temp_dir = tempfile::tempdir().expect("tempdir");
    let config = config_for(temp_dir.path());
    let base = solid_frame([0, 0, 0, 255]);
    let mut changed = base.clone();
    for y in 0..4 {
        for x in 0..4 {
            changed.set_pixel(x, y, [255, 0, 0, 255]);
        }
    }

    let capture = MockCapture::new(
        dimensions(),
        vec![captured(1_000, base), captured(2_000, changed)],
    );

    let recorder = Recorder::new(config, "session-changed", capture);
    let storage = recorder.run().expect("run recorder");

    let patches = load_patch_index(storage.layout().index_dir()).expect("load patch index");
    assert_eq!(patches.len(), 1);
}

#[test]
fn writes_periodic_keyframes_without_changes() {
    let temp_dir = tempfile::tempdir().expect("tempdir");
    let config = config_for(temp_dir.path());
    let frame = solid_frame([10, 10, 10, 255]);

    let capture = MockCapture::new(
        dimensions(),
        vec![captured(1_000, frame.clone()), captured(2_500, frame)],
    );

    let recorder = Recorder::new(config, "session-keyframes", capture);
    let storage = recorder.run().expect("run recorder");

    let keyframes = load_keyframe_index(storage.layout().index_dir()).expect("load keyframe index");
    assert_eq!(keyframes.len(), 2);

    let patches = load_patch_index(storage.layout().index_dir()).expect("load patch index");
    assert!(patches.is_empty());
}

#[test]
fn finalizes_manifest_on_shutdown() {
    let temp_dir = tempfile::tempdir().expect("tempdir");
    let config = config_for(temp_dir.path());
    let frame = solid_frame([0, 0, 0, 255]);

    let capture = MockCapture::new(
        dimensions(),
        vec![captured(1_000, frame.clone()), captured(1_500, frame)],
    );

    let recorder = Recorder::new(config, "session-finalize", capture);
    let storage = recorder.run().expect("run recorder");
    let manifest = storage.load_manifest().expect("load manifest");

    assert_eq!(manifest.finished_at, Some(1_500));
}

#[test]
fn run_until_stops_after_requested_iteration() {
    let temp_dir = tempfile::tempdir().expect("tempdir");
    let config = config_for(temp_dir.path());
    let base = solid_frame([0, 0, 0, 255]);
    let mut changed = base.clone();
    for y in 0..4 {
        for x in 0..4 {
            changed.set_pixel(x, y, [255, 0, 0, 255]);
        }
    }

    let capture = MockCapture::new(
        dimensions(),
        vec![
            captured(1_000, base),
            captured(2_000, changed.clone()),
            captured(3_000, changed),
        ],
    );

    let recorder = Recorder::new(config, "session-stop", capture);
    let mut stop_calls = 0;
    let storage = recorder
        .run_until(|| {
            stop_calls += 1;
            stop_calls > 1
        })
        .expect("run recorder");

    let patches = load_patch_index(storage.layout().index_dir()).expect("load patch index");
    assert_eq!(patches.len(), 1);

    let manifest = storage.load_manifest().expect("load manifest");
    assert_eq!(manifest.finished_at, Some(2_000));
}

#[test]
fn identical_frames_skip_diff_but_still_finalize_cleanly() {
    let temp_dir = tempfile::tempdir().expect("tempdir");
    let mut config = config_for(temp_dir.path());
    config.keyframe_interval_ms = 10_000;
    let frame = solid_frame([5, 5, 5, 255]);

    let capture = MockCapture::new(
        dimensions(),
        vec![
            captured(1_000, frame.clone()),
            captured(2_000, frame.clone()),
            captured(3_000, frame),
        ],
    );

    let recorder = Recorder::new(config, "session-identical", capture);
    let mut stop_calls = 0;
    let storage = recorder
        .run_until(|| {
            stop_calls += 1;
            stop_calls > 2
        })
        .expect("run recorder");

    let patches = load_patch_index(storage.layout().index_dir()).expect("load patch index");
    assert!(patches.is_empty());

    let keyframes = load_keyframe_index(storage.layout().index_dir()).expect("load keyframe index");
    assert_eq!(keyframes.len(), 1);

    let manifest = storage.load_manifest().expect("load manifest");
    assert_eq!(manifest.finished_at, Some(3_000));
}

#[test]
fn small_changes_still_write_patches_with_detailed_sampling() {
    let temp_dir = tempfile::tempdir().expect("tempdir");
    let config = config_for(temp_dir.path());
    let base = solid_frame([0, 0, 0, 255]);
    let mut changed = base.clone();
    changed.set_pixel(3, 3, [255, 255, 255, 255]);

    let capture = MockCapture::new(
        dimensions(),
        vec![captured(1_000, base), captured(2_000, changed)],
    );

    let recorder = Recorder::new(config, "session-small-change", capture);
    let storage = recorder.run().expect("run recorder");

    let patches = load_patch_index(storage.layout().index_dir()).expect("load patch index");
    assert_eq!(patches.len(), 1);
}

#[test]
fn run_until_with_stats_reports_skip_and_patch_counts() {
    let temp_dir = tempfile::tempdir().expect("tempdir");
    let mut config = config_for(temp_dir.path());
    config.keyframe_interval_ms = 10_000;

    let base = solid_frame([0, 0, 0, 255]);
    let mut changed = base.clone();
    for y in 0..4 {
        for x in 0..4 {
            changed.set_pixel(x, y, [255, 0, 0, 255]);
        }
    }

    let capture = MockCapture::new(
        dimensions(),
        vec![
            captured(1_000, base.clone()),
            captured(2_000, base),
            captured(3_000, changed),
        ],
    );

    let recorder = Recorder::new(config, "session-stats", capture);
    let (_storage, stats) = recorder
        .run_until_with_stats(|| false)
        .expect("run recorder");

    assert_eq!(
        stats,
        RecordingStats {
            frames_seen: 3,
            identical_frames_skipped: 1,
            sampled_precheck_skipped: 0,
            diff_runs: 1,
            patch_frames_written: 1,
            patch_regions_written: 1,
            keyframes_written: 1,
            started_at: 1_000,
            finished_at: 3_000,
        }
    );
}

#[test]
fn pause_signal_holds_capture_until_stop_requested() {
    let temp_dir = tempfile::tempdir().expect("tempdir");
    let mut config = config_for(temp_dir.path());
    config.sampling_interval_ms = 1;
    let pause_signal = temp_dir
        .path()
        .join("sessions")
        .join("session-paused")
        .join("pause.signal");
    std::fs::create_dir_all(pause_signal.parent().expect("pause dir")).expect("pause dir");
    std::fs::write(&pause_signal, b"pause").expect("write pause signal");

    let base = solid_frame([0, 0, 0, 255]);
    let capture = MockCapture::new(
        dimensions(),
        vec![captured(1_000, base.clone()), captured(2_000, base)],
    );

    let recorder = Recorder::new(config, "session-paused", capture);
    let mut stop_calls = 0;
    let (_storage, stats) = recorder
        .run_until_with_stats(|| {
            stop_calls += 1;
            stop_calls > 2
        })
        .expect("run recorder");

    let status_contents =
        std::fs::read_to_string(temp_dir.path().join("sessions/session-paused/status.json"))
            .expect("read status");
    let status: SessionStatus = serde_json::from_str(&status_contents).expect("parse status");

    assert_eq!(status.state, SessionState::Stopped);
    assert_eq!(stats.frames_seen, 1);
    assert_eq!(stats.finished_at, 1_000);
}
