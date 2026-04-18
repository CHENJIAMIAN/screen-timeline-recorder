use std::fs;
use std::path::PathBuf;

use screen_timeline_recorder::config::RecorderConfig;
use screen_timeline_recorder::diff::PatchRegion;
use screen_timeline_recorder::index::{KeyframeIndexEntry, PatchIndexEntry, load_keyframe_index};
use screen_timeline_recorder::recorder::RecordingStats;
use screen_timeline_recorder::session::{Manifest, SessionLayout, SessionState, SessionStatus};
use screen_timeline_recorder::storage::{SessionDimensions, Storage};

fn new_config(output_dir: &std::path::Path) -> RecorderConfig {
    RecorderConfig::default().with_output_dir(output_dir.to_path_buf())
}

fn start_storage(output_dir: &std::path::Path) -> Storage {
    let config = new_config(output_dir);
    Storage::start_session(
        config,
        "2026-04-13",
        1_700_000_000_000,
        SessionDimensions {
            display_width: 1920,
            display_height: 1080,
            working_width: 960,
            working_height: 540,
        },
    )
    .expect("start session")
}

#[test]
fn creates_canonical_session_layout() {
    let temp_dir = tempfile::tempdir().expect("tempdir");
    let storage = start_storage(temp_dir.path());

    let layout = storage.layout();
    assert!(
        layout
            .root()
            .ends_with(PathBuf::from("sessions").join("2026-04-13"))
    );
    assert!(layout.manifest_path().exists());
    assert!(layout.status_path().exists());
    assert!(
        layout.stop_signal_path().ends_with(
            PathBuf::from("sessions")
                .join("2026-04-13")
                .join("stop.signal")
        )
    );
    assert!(
        layout.pause_signal_path().ends_with(
            PathBuf::from("sessions")
                .join("2026-04-13")
                .join("pause.signal")
        )
    );
    assert!(layout.keyframes_dir().exists());
    assert!(layout.patches_dir().exists());
    assert!(layout.index_dir().exists());
}

#[test]
fn creates_layout_under_configured_output_dir() {
    let temp_dir = tempfile::tempdir().expect("tempdir");
    let custom_output = temp_dir.path().join("custom-output");
    let storage = start_storage(&custom_output);

    let layout = storage.layout();
    assert!(layout.root().starts_with(custom_output));
}

#[test]
fn persists_manifest_with_required_fields() {
    let temp_dir = tempfile::tempdir().expect("tempdir");
    let storage = start_storage(temp_dir.path());

    let manifest_contents =
        fs::read_to_string(storage.layout().manifest_path()).expect("read manifest");
    let manifest: Manifest = serde_json::from_str(&manifest_contents).expect("parse manifest");

    assert_eq!(manifest.session_id, "2026-04-13");
    assert_eq!(manifest.started_at, 1_700_000_000_000);
    assert_eq!(manifest.display_width, 1920);
    assert_eq!(manifest.display_height, 1080);
    assert_eq!(manifest.working_width, 960);
    assert_eq!(manifest.working_height, 540);
    assert_eq!(manifest.sampling_interval_ms, 100);
    assert_eq!(manifest.block_width, 16);
    assert_eq!(manifest.block_height, 16);
    assert_eq!(manifest.keyframe_interval_ms, 30_000);
    assert_eq!(manifest.sensitivity_mode, "balanced");
    assert_eq!(manifest.precheck_threshold, 0.01);
    assert_eq!(manifest.block_difference_threshold, 0.05);
    assert_eq!(manifest.changed_pixel_ratio_threshold, 0.0);
    assert_eq!(manifest.stability_window, 2);
    assert_eq!(manifest.compression_format, "png");
    assert_eq!(manifest.recorder_version, env!("CARGO_PKG_VERSION"));
    assert_eq!(manifest.viewer_default_zoom, 1.0);
    assert!(manifest.viewer_overlay_enabled_by_default);
    assert!(manifest.burn_in_enabled);
    assert_eq!(
        manifest.viewer_language,
        screen_timeline_recorder::config::ViewerLanguage::Auto
    );
}

#[test]
fn writes_keyframe_and_index_entry() {
    let temp_dir = tempfile::tempdir().expect("tempdir");
    let mut storage = start_storage(temp_dir.path());

    let payload = vec![1_u8, 2, 3, 4].repeat(960 * 540);
    let path = storage
        .write_keyframe(1_700_000_000_123, &payload)
        .expect("write keyframe");

    assert!(path.exists());

    let entries = load_keyframe_index(storage.layout().index_dir()).expect("load keyframe index");
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].timestamp_ms, 1_700_000_000_123);
    assert_eq!(entries[0].path, "keyframes/1700000000123.png");
}

#[test]
fn writes_patch_and_index_entry() {
    let temp_dir = tempfile::tempdir().expect("tempdir");
    let mut storage = start_storage(temp_dir.path());

    let patch = PatchRegion {
        x: 0,
        y: 0,
        width: 32,
        height: 32,
        data: vec![5_u8, 6, 7, 255].repeat(32 * 32),
    };

    let path = storage
        .write_patches(1_700_000_000_500, &[patch])
        .expect("write patch");

    assert_eq!(path.len(), 1);
    assert!(path[0].exists());

    let patches_path = storage.layout().index_dir().join("patches.jsonl");
    let contents = fs::read_to_string(patches_path).expect("read patches index");
    let line = contents.lines().next().expect("patch index line");
    let entry: PatchIndexEntry = serde_json::from_str(line).expect("parse patch index");

    assert_eq!(entry.timestamp_ms, 1_700_000_000_500);
    assert_eq!(entry.sequence, 0);
    assert_eq!(entry.path, "patches/1700000000500_0.stp");
}

#[test]
fn keyframes_are_losslessly_compressed_for_new_sessions() {
    let temp_dir = tempfile::tempdir().expect("tempdir");
    let mut storage = start_storage(temp_dir.path());

    let payload = vec![42_u8; 960 * 540 * 4];
    let path = storage
        .write_keyframe(1_700_000_000_123, &payload)
        .expect("write keyframe");

    let metadata = fs::metadata(&path).expect("keyframe metadata");
    assert!(metadata.len() < payload.len() as u64);
}

#[test]
fn no_op_write_for_empty_patch_list() {
    let temp_dir = tempfile::tempdir().expect("tempdir");
    let mut storage = start_storage(temp_dir.path());

    let written = storage
        .write_patches(1_700_000_001_000, &[])
        .expect("write patches");
    assert!(written.is_empty());

    let patches_path = storage.layout().index_dir().join("patches.jsonl");
    if patches_path.exists() {
        let contents = fs::read_to_string(patches_path).expect("read patches index");
        assert!(contents.trim().is_empty());
    }
}

#[test]
fn coalesces_rapid_repeated_region_writes() {
    let temp_dir = tempfile::tempdir().expect("tempdir");
    let mut storage = start_storage(temp_dir.path());

    let patch = PatchRegion {
        x: 10,
        y: 20,
        width: 16,
        height: 16,
        data: vec![9_u8, 9, 9, 255].repeat(16 * 16),
    };

    storage
        .write_patches(1_700_000_001_000, &[patch.clone()])
        .expect("write patch");
    storage
        .write_patches(1_700_000_001_050, &[patch])
        .expect("write patch");

    let patches_path = storage.layout().index_dir().join("patches.jsonl");
    let contents = fs::read_to_string(patches_path).expect("read patches index");
    assert_eq!(contents.lines().count(), 1);
}

#[test]
fn does_not_coalesce_repeated_region_writes_when_pixel_data_changes() {
    let temp_dir = tempfile::tempdir().expect("tempdir");
    let mut storage = start_storage(temp_dir.path());

    let first_patch = PatchRegion {
        x: 10,
        y: 20,
        width: 16,
        height: 16,
        data: vec![9_u8, 9, 9, 255].repeat(16 * 16),
    };
    let second_patch = PatchRegion {
        x: 10,
        y: 20,
        width: 16,
        height: 16,
        data: vec![200_u8, 1, 1, 255].repeat(16 * 16),
    };

    storage
        .write_patches(1_700_000_001_000, &[first_patch])
        .expect("write first patch");
    storage
        .write_patches(1_700_000_001_050, &[second_patch])
        .expect("write second patch");

    let patches_path = storage.layout().index_dir().join("patches.jsonl");
    let contents = fs::read_to_string(patches_path).expect("read patches index");
    assert_eq!(
        contents.lines().count(),
        2,
        "same region with different pixels must not be coalesced"
    );
}

#[test]
fn index_lookup_uses_index_files() {
    let temp_dir = tempfile::tempdir().expect("tempdir");
    let layout = SessionLayout::new(temp_dir.path(), "2026-04-13");
    layout.create_dirs().expect("create dirs");

    let entry = KeyframeIndexEntry {
        timestamp_ms: 42,
        path: "keyframes/ghost.bin".to_string(),
    };
    let index_path = layout.index_dir().join("keyframes.jsonl");
    let line = serde_json::to_string(&entry).expect("serialize entry");
    fs::write(index_path, format!("{line}\n")).expect("write index");

    fs::remove_dir_all(layout.keyframes_dir()).expect("remove keyframes dir");

    let entries = load_keyframe_index(layout.index_dir()).expect("load index");
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].path, "keyframes/ghost.bin");
}

#[test]
fn persists_status_updates_for_in_progress_session() {
    let temp_dir = tempfile::tempdir().expect("tempdir");
    let mut storage = start_storage(temp_dir.path());
    let stats = RecordingStats {
        frames_seen: 3,
        identical_frames_skipped: 1,
        sampled_precheck_skipped: 1,
        diff_runs: 1,
        patch_frames_written: 1,
        patch_regions_written: 2,
        keyframes_written: 1,
        started_at: 1_700_000_000_000,
        finished_at: 1_700_000_001_500,
    };

    storage
        .write_status(SessionState::Running, &stats)
        .expect("write in-progress status");

    let contents = fs::read_to_string(storage.layout().status_path()).expect("read status");
    let status: SessionStatus = serde_json::from_str(&contents).expect("parse status");
    assert_eq!(status.state, SessionState::Running);
    assert!(status.recording);
    assert_eq!(status.stats, stats);
}

#[test]
fn persists_paused_status_state() {
    let temp_dir = tempfile::tempdir().expect("tempdir");
    let mut storage = start_storage(temp_dir.path());
    let stats = RecordingStats {
        frames_seen: 3,
        identical_frames_skipped: 1,
        sampled_precheck_skipped: 1,
        diff_runs: 1,
        patch_frames_written: 1,
        patch_regions_written: 2,
        keyframes_written: 1,
        started_at: 1_700_000_000_000,
        finished_at: 1_700_000_001_500,
    };

    storage
        .write_status(SessionState::Paused, &stats)
        .expect("write paused status");

    let contents = fs::read_to_string(storage.layout().status_path()).expect("read status");
    let status: SessionStatus = serde_json::from_str(&contents).expect("parse status");
    assert_eq!(status.state, SessionState::Paused);
    assert!(status.recording);
    assert_eq!(status.stats, stats);
}

#[test]
fn finalizing_session_marks_status_as_complete() {
    let temp_dir = tempfile::tempdir().expect("tempdir");
    let mut storage = start_storage(temp_dir.path());

    storage
        .finalize_session(1_700_000_002_000)
        .expect("finalize");

    let contents = fs::read_to_string(storage.layout().status_path()).expect("read status");
    let status: SessionStatus = serde_json::from_str(&contents).expect("parse status");
    assert_eq!(status.state, SessionState::Stopped);
    assert!(!status.recording);
    assert_eq!(status.stats.finished_at, 1_700_000_002_000);
}
