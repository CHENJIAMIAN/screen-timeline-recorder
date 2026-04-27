use std::path::Path;

use screen_timeline_recorder::{
    config::RecorderConfig,
    session::SessionLayout,
    video_recorder::{
        build_ffmpeg_segment_args, build_segment_entries_for_run, build_video_segment_index,
        classify_ffmpeg_exit, is_ignorable_ffmpeg_stop_error, resolve_ffmpeg_path,
    },
};

#[test]
fn resolve_ffmpeg_path_prefers_bundled_binary_next_to_exe() {
    let temp_dir = tempfile::tempdir().expect("tempdir");
    let exe_dir = temp_dir.path().join("dist");
    std::fs::create_dir_all(exe_dir.join("ffmpeg")).expect("ffmpeg dir");
    let bundled = exe_dir.join("ffmpeg").join("ffmpeg.exe");
    std::fs::write(&bundled, b"binary").expect("write fake ffmpeg");

    let resolved = resolve_ffmpeg_path(Some(&exe_dir), &[]).expect("resolve ffmpeg");

    assert_eq!(resolved, bundled);
}

#[test]
fn resolve_ffmpeg_path_accepts_explicit_candidates() {
    let temp_dir = tempfile::tempdir().expect("tempdir");
    let candidate = temp_dir.path().join("custom-ffmpeg.exe");
    std::fs::write(&candidate, b"binary").expect("write fake ffmpeg");

    let resolved = resolve_ffmpeg_path(None, &[candidate.clone()]).expect("resolve ffmpeg");

    assert_eq!(resolved, candidate);
}

#[test]
fn resolve_ffmpeg_path_falls_back_to_repo_tools_ffmpeg_for_dev_builds() {
    let temp_dir = tempfile::tempdir().expect("tempdir");
    let exe_dir = temp_dir.path().join("target").join("debug");
    let repo_root = temp_dir.path();
    std::fs::create_dir_all(repo_root.join("tools").join("ffmpeg")).expect("ffmpeg dir");
    std::fs::create_dir_all(&exe_dir).expect("exe dir");
    let bundled = repo_root.join("tools").join("ffmpeg").join("ffmpeg.exe");
    std::fs::write(&bundled, b"binary").expect("write fake ffmpeg");

    let resolved = resolve_ffmpeg_path(Some(&exe_dir), &[]).expect("resolve ffmpeg");

    assert_eq!(resolved, bundled);
}

#[test]
fn build_ffmpeg_segment_args_targets_segmented_mp4_output() {
    let temp_dir = tempfile::tempdir().expect("tempdir");
    let config = RecorderConfig::default().with_output_dir(temp_dir.path().to_path_buf());
    let layout = SessionLayout::new(temp_dir.path(), "session-video");

    let args = build_ffmpeg_segment_args(
        &config,
        &layout,
        1920,
        1080,
        1440,
        810,
        30,
        0,
        Path::new("ffmpeg.exe"),
    );

    let joined = args.join(" ");
    assert!(joined.contains("-f gdigrab"));
    assert!(joined.contains("-framerate 10"));
    assert!(joined.contains("-video_size 1920x1080"));
    assert!(joined.contains("-rtbufsize 32M"));
    assert!(joined.contains("-vf scale=1440:810"));
    assert!(joined.contains("-tune zerolatency"));
    assert!(joined.contains("-threads 2"));
    assert!(joined.contains("-x264-params rc-lookahead=0:sync-lookahead=0"));
    assert!(joined.contains("-f segment"));
    assert!(joined.contains("-segment_time 30"));
    assert!(joined.contains("segments\\run-000000-%06d.mp4") || joined.contains("segments/run-000000-%06d.mp4"));
}

#[test]
fn build_ffmpeg_segment_args_adds_burn_in_filter_when_enabled() {
    let temp_dir = tempfile::tempdir().expect("tempdir");
    let config = RecorderConfig::default().with_output_dir(temp_dir.path().to_path_buf());
    let layout = SessionLayout::new(temp_dir.path(), "session-video");

    let args = build_ffmpeg_segment_args(
        &config,
        &layout,
        1920,
        1080,
        1920,
        1080,
        30,
        0,
        Path::new("ffmpeg.exe"),
    );

    let joined = args.join(" ");
    assert!(joined.contains("drawtext="));
    assert!(joined.contains("localtime"));
    assert!(joined.contains("%H-%M-%S"));
}

#[test]
fn build_ffmpeg_segment_args_omits_burn_in_filter_when_disabled() {
    let temp_dir = tempfile::tempdir().expect("tempdir");
    let mut config = RecorderConfig::default().with_output_dir(temp_dir.path().to_path_buf());
    config.burn_in_enabled = false;
    let layout = SessionLayout::new(temp_dir.path(), "session-video");

    let args = build_ffmpeg_segment_args(
        &config,
        &layout,
        1920,
        1080,
        1920,
        1080,
        30,
        0,
        Path::new("ffmpeg.exe"),
    );

    let joined = args.join(" ");
    assert!(!joined.contains("drawtext="));
}

#[test]
fn build_video_segment_index_uses_segment_duration_and_file_sizes() {
    let temp_dir = tempfile::tempdir().expect("tempdir");
    let layout = SessionLayout::new(temp_dir.path(), "session-video");
    std::fs::create_dir_all(layout.segments_dir()).expect("segments dir");
    std::fs::write(layout.segments_dir().join("000000.mp4"), vec![0u8; 11]).expect("seg0");
    std::fs::write(layout.segments_dir().join("000001.mp4"), vec![0u8; 17]).expect("seg1");

    let entries = build_video_segment_index(&layout, 1_000, 45_000, 30_000).expect("index");

    assert_eq!(entries.len(), 2);
    assert_eq!(entries[0].sequence, 0);
    assert_eq!(entries[0].started_at, 1_000);
    assert_eq!(entries[0].finished_at, Some(31_000));
    assert_eq!(entries[0].relative_path, "segments/000000.mp4");
    assert_eq!(entries[0].bytes, 11);
    assert_eq!(entries[1].sequence, 1);
    assert_eq!(entries[1].started_at, 31_000);
    assert_eq!(entries[1].finished_at, Some(45_000));
    assert_eq!(entries[1].bytes, 17);
}

#[test]
fn video_layout_creation_only_builds_video_directories() {
    let temp_dir = tempfile::tempdir().expect("tempdir");
    let layout = SessionLayout::new(temp_dir.path(), "session-video");

    layout.create_video_dirs().expect("create video dirs");

    assert!(layout.root().exists());
    assert!(layout.segments_dir().exists());
    assert!(layout.index_dir().exists());
}

#[test]
fn build_segment_entries_for_run_keeps_timeline_continuous_across_pause_resume() {
    let entries = build_segment_entries_for_run(
        1_000,
        31_000,
        30_000,
        0,
        &[
            ("000000.mp4".to_string(), 11),
            ("000001.mp4".to_string(), 13),
        ],
    );

    assert_eq!(entries.len(), 2);
    assert_eq!(entries[0].sequence, 0);
    assert_eq!(entries[0].started_at, 1_000);
    assert_eq!(entries[0].finished_at, Some(31_000));
    assert_eq!(entries[1].sequence, 1);
    assert_eq!(entries[1].started_at, 31_000);
    assert_eq!(entries[1].finished_at, Some(32_000));
}

#[test]
fn classify_ffmpeg_exit_treats_non_zero_exit_during_pause_as_pause() {
    assert_eq!(classify_ffmpeg_exit(false, true), Ok(true));
    assert_eq!(classify_ffmpeg_exit(true, false), Ok(false));
    assert!(classify_ffmpeg_exit(false, false).is_err());
}

#[test]
fn is_ignorable_ffmpeg_stop_error_accepts_broken_pipe_and_invalid_input() {
    assert!(is_ignorable_ffmpeg_stop_error(&std::io::Error::from(
        std::io::ErrorKind::BrokenPipe
    )));
    assert!(is_ignorable_ffmpeg_stop_error(&std::io::Error::from(
        std::io::ErrorKind::InvalidInput
    )));
    assert!(!is_ignorable_ffmpeg_stop_error(&std::io::Error::from(
        std::io::ErrorKind::PermissionDenied
    )));
}
