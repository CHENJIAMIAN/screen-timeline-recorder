use screen_timeline_recorder::{
    config::RecorderConfig,
    diff::PatchRegion,
    frame::Frame,
    reconstruct::Reconstructor,
    session::Manifest,
    storage::{SessionDimensions, Storage},
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
fn reconstructs_from_keyframe_alone() {
    let temp_dir = tempfile::tempdir().expect("tempdir");
    let mut storage = start_storage(temp_dir.path());
    let frame = Frame::solid_rgba(4, 4, [7, 8, 9, 255]);
    storage
        .write_keyframe(100, frame.as_rgba())
        .expect("write keyframe");

    let reconstructor = Reconstructor::open(temp_dir.path(), "2026-04-13").expect("open");
    let reconstructed = reconstructor.reconstruct_at(100).expect("reconstruct");

    assert_eq!(reconstructed, frame);
}

#[test]
fn reconstructs_after_multiple_patches() {
    let temp_dir = tempfile::tempdir().expect("tempdir");
    let mut storage = start_storage(temp_dir.path());
    let base = Frame::solid_rgba(4, 4, [0, 0, 0, 255]);
    storage
        .write_keyframe(100, base.as_rgba())
        .expect("write keyframe");

    storage
        .write_patches(
            110,
            &[PatchRegion {
                x: 0,
                y: 0,
                width: 2,
                height: 2,
                data: vec![255, 0, 0, 255].repeat(4),
            }],
        )
        .expect("write patch");
    storage
        .write_patches(
            120,
            &[PatchRegion {
                x: 2,
                y: 2,
                width: 2,
                height: 2,
                data: vec![0, 255, 0, 255].repeat(4),
            }],
        )
        .expect("write patch");

    let reconstructor = Reconstructor::open(temp_dir.path(), "2026-04-13").expect("open");
    let reconstructed = reconstructor.reconstruct_at(120).expect("reconstruct");

    let mut expected = base.clone();
    for y in 0..2 {
        for x in 0..2 {
            expected.set_pixel(x, y, [255, 0, 0, 255]);
        }
    }
    for y in 2..4 {
        for x in 2..4 {
            expected.set_pixel(x, y, [0, 255, 0, 255]);
        }
    }

    assert_eq!(reconstructed, expected);
}

#[test]
fn selects_nearest_prior_keyframe() {
    let temp_dir = tempfile::tempdir().expect("tempdir");
    let mut storage = start_storage(temp_dir.path());
    let earlier = Frame::solid_rgba(4, 4, [10, 0, 0, 255]);
    let later = Frame::solid_rgba(4, 4, [0, 10, 0, 255]);
    storage
        .write_keyframe(100, earlier.as_rgba())
        .expect("write keyframe");
    storage
        .write_keyframe(200, later.as_rgba())
        .expect("write keyframe");

    let reconstructor = Reconstructor::open(temp_dir.path(), "2026-04-13").expect("open");
    let reconstructed = reconstructor.reconstruct_at(150).expect("reconstruct");

    assert_eq!(reconstructed, earlier);
}

#[test]
fn replays_patches_in_timestamp_and_sequence_order() {
    let temp_dir = tempfile::tempdir().expect("tempdir");
    let mut storage = start_storage(temp_dir.path());
    let base = Frame::solid_rgba(4, 4, [0, 0, 0, 255]);
    storage
        .write_keyframe(100, base.as_rgba())
        .expect("write keyframe");

    storage
        .write_patches(
            110,
            &[
                PatchRegion {
                    x: 0,
                    y: 0,
                    width: 2,
                    height: 2,
                    data: vec![1, 0, 0, 255].repeat(4),
                },
                PatchRegion {
                    x: 0,
                    y: 0,
                    width: 2,
                    height: 2,
                    data: vec![2, 0, 0, 255].repeat(4),
                },
            ],
        )
        .expect("write patches");

    let reconstructor = Reconstructor::open(temp_dir.path(), "2026-04-13").expect("open");
    let reconstructed = reconstructor.reconstruct_at(110).expect("reconstruct");

    for y in 0..2 {
        for x in 0..2 {
            assert_eq!(reconstructed.pixel(x, y), [2, 0, 0, 255]);
        }
    }
}

#[test]
fn reconstructs_legacy_raw_session_data() {
    let temp_dir = tempfile::tempdir().expect("tempdir");
    let session_root = temp_dir.path().join("sessions").join("legacy-raw");
    std::fs::create_dir_all(session_root.join("keyframes")).expect("create keyframes");
    std::fs::create_dir_all(session_root.join("patches")).expect("create patches");
    std::fs::create_dir_all(session_root.join("index")).expect("create index");

    let manifest = Manifest {
        session_id: "legacy-raw".to_string(),
        started_at: 100,
        finished_at: Some(120),
        display_width: 4,
        display_height: 4,
        working_width: 4,
        working_height: 4,
        sampling_interval_ms: 500,
        block_width: 32,
        block_height: 32,
        keyframe_interval_ms: 60_000,
        sensitivity_mode: "balanced".to_string(),
        precheck_threshold: 0.01,
        block_difference_threshold: 0.05,
        changed_pixel_ratio_threshold: 0.1,
        stability_window: 2,
        compression_format: "raw".to_string(),
        recorder_version: "0.1.0".to_string(),
        viewer_default_zoom: 1.0,
        viewer_overlay_enabled_by_default: true,
        burn_in_enabled: true,
        viewer_language: screen_timeline_recorder::config::ViewerLanguage::Auto,
    };
    std::fs::write(
        session_root.join("manifest.json"),
        serde_json::to_string_pretty(&manifest).expect("serialize manifest"),
    )
    .expect("write manifest");

    let base = Frame::solid_rgba(4, 4, [0, 0, 0, 255]);
    std::fs::write(
        session_root.join("keyframes").join("100.bin"),
        base.as_rgba(),
    )
    .expect("write raw keyframe");
    std::fs::write(
        session_root.join("index").join("keyframes.jsonl"),
        "{\"timestamp_ms\":100,\"path\":\"keyframes/100.bin\"}\n",
    )
    .expect("write keyframe index");

    let legacy_patch = serde_json::json!({
        "x": 0,
        "y": 0,
        "width": 2,
        "height": 2,
        "data": vec![255, 0, 0, 255].repeat(4),
    });
    std::fs::write(
        session_root.join("patches").join("110_0.bin"),
        serde_json::to_vec(&legacy_patch).expect("serialize patch"),
    )
    .expect("write patch");
    std::fs::write(
        session_root.join("index").join("patches.jsonl"),
        "{\"timestamp_ms\":110,\"sequence\":0,\"path\":\"patches/110_0.bin\"}\n",
    )
    .expect("write patch index");

    let reconstructor = Reconstructor::open(temp_dir.path(), "legacy-raw").expect("open");
    let reconstructed = reconstructor.reconstruct_at(110).expect("reconstruct");

    for y in 0..2 {
        for x in 0..2 {
            assert_eq!(reconstructed.pixel(x, y), [255, 0, 0, 255]);
        }
    }
}
