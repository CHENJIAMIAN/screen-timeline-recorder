use std::fs;

use screen_timeline_recorder::config::RecorderConfig;
use screen_timeline_recorder::diff::PatchRegion;
use screen_timeline_recorder::frame::Frame;
use screen_timeline_recorder::logging::StructuredError;
use screen_timeline_recorder::reconstruct::Reconstructor;
use screen_timeline_recorder::session::Manifest;
use screen_timeline_recorder::storage::{SessionDimensions, Storage, StorageError};

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
fn storage_failure_surfaces_structured_error() {
    let temp_dir = tempfile::tempdir().expect("tempdir");
    let blocked_path = temp_dir.path().join("blocked-output");
    fs::write(&blocked_path, b"not a directory").expect("write blocked path");

    let config = RecorderConfig::default().with_output_dir(blocked_path.clone());
    let err = Storage::start_session(
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
    .expect_err("expected storage error");

    match err {
        StorageError::Structured(structured) => assert_structured_fields(&structured),
        other => panic!("expected structured storage error, got {other:?}"),
    }
}

#[test]
fn corrupted_patch_does_not_brick_session_reconstruction() {
    let temp_dir = tempfile::tempdir().expect("tempdir");
    let mut storage = start_storage(temp_dir.path());
    let base = Frame::solid_rgba(4, 4, [0, 0, 0, 255]);
    storage
        .write_keyframe(100, base.as_rgba())
        .expect("write keyframe");

    storage
        .write_patches(
            120,
            &[PatchRegion {
                x: 0,
                y: 0,
                width: 2,
                height: 2,
                data: vec![255, 0, 0, 255].repeat(4),
            }],
        )
        .expect("write patch");

    let patch_index_path = storage.layout().index_dir().join("patches.jsonl");
    let patch_line = fs::read_to_string(&patch_index_path)
        .expect("read patch index")
        .lines()
        .next()
        .expect("patch index line")
        .to_string();
    let entry: screen_timeline_recorder::index::PatchIndexEntry =
        serde_json::from_str(&patch_line).expect("parse patch index");
    let patch_path = storage.layout().root().join(entry.path);
    fs::write(&patch_path, b"corrupted payload").expect("corrupt patch");

    let reconstructor = Reconstructor::open(temp_dir.path(), "2026-04-13").expect("open");
    let reconstructed = reconstructor.reconstruct_at(120).expect("reconstruct");

    assert_eq!(reconstructed, base);
}

#[test]
fn finalizes_manifest_after_recoverable_storage_errors() {
    let temp_dir = tempfile::tempdir().expect("tempdir");
    let mut storage = start_storage(temp_dir.path());
    let patches_dir = storage.layout().patches_dir().to_path_buf();
    fs::remove_dir_all(&patches_dir).expect("remove patches dir");

    let patch = PatchRegion {
        x: 0,
        y: 0,
        width: 2,
        height: 2,
        data: vec![1, 2, 3, 4],
    };

    let err = storage
        .write_patches(1_700_000_000_500, &[patch])
        .expect_err("expected write error");

    if let StorageError::Structured(structured) = err {
        assert_structured_fields(&structured);
    } else {
        panic!("expected structured error after patch write failure");
    }

    storage
        .finalize_session(1_700_000_000_500)
        .expect("finalize session");
    let manifest = Manifest::load(storage.layout().manifest_path()).expect("load manifest");
    assert_eq!(manifest.finished_at, Some(1_700_000_000_500));
}

fn assert_structured_fields(structured: &StructuredError) {
    assert!(!structured.operation.is_empty());
    assert!(structured.path.is_some());
    assert!(!structured.kind.is_empty());
    assert!(!structured.message.is_empty());
}
