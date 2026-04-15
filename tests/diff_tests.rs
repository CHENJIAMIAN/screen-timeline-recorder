use screen_timeline_recorder::{
    config::{RecorderConfig, SensitivityMode},
    diff::DiffEngine,
    frame::Frame,
};

fn fill_rect(frame: &mut Frame, x: usize, y: usize, width: usize, height: usize, rgba: [u8; 4]) {
    for row in y..(y + height) {
        for col in x..(x + width) {
            frame.set_pixel(col, row, rgba);
        }
    }
}

#[test]
fn unchanged_frames_produce_no_patches() {
    let config = RecorderConfig::default();
    let mut engine = DiffEngine::new(&config);
    let frame = Frame::solid_rgba(8, 8, [0, 0, 0, 255]);

    let result = engine.diff(&frame, &frame).expect("diff result");

    assert!(result.patches.is_empty());
    assert!(result.precheck_skipped);
}

#[test]
fn low_resolution_precheck_skips_block_diff_for_nearly_identical_frames() {
    let config = RecorderConfig::default();
    let mut engine = DiffEngine::new(&config);
    let previous = Frame::solid_rgba(8, 8, [0, 0, 0, 255]);
    let mut current = previous.clone();
    current.set_pixel(0, 0, [1, 1, 1, 255]);

    let result = engine.diff(&previous, &current).expect("diff result");

    assert!(result.precheck_skipped);
    assert!(result.patches.is_empty());
}

#[test]
fn single_changed_block_yields_one_patch() {
    let mut config = RecorderConfig::default();
    config.block_width = 4;
    config.block_height = 4;
    config.sensitivity_mode = SensitivityMode::Detailed;
    let mut engine = DiffEngine::new(&config);
    let previous = Frame::solid_rgba(8, 8, [0, 0, 0, 255]);
    let mut current = previous.clone();
    fill_rect(&mut current, 0, 0, 4, 4, [255, 0, 0, 255]);

    let result = engine.diff(&previous, &current).expect("diff result");

    assert!(!result.precheck_skipped);
    assert_eq!(result.patches.len(), 1);
    let patch = &result.patches[0];
    assert_eq!(patch.x, 0);
    assert_eq!(patch.y, 0);
    assert_eq!(patch.width, 4);
    assert_eq!(patch.height, 4);
}

#[test]
fn tiny_sub_threshold_changes_are_ignored() {
    let mut config = RecorderConfig::default();
    config.block_width = 4;
    config.block_height = 4;
    config.sensitivity_mode = SensitivityMode::Conservative;
    let mut engine = DiffEngine::new(&config);
    let previous = Frame::solid_rgba(8, 8, [0, 0, 0, 255]);
    let mut current = previous.clone();
    current.set_pixel(0, 0, [8, 8, 8, 255]);
    current.set_pixel(1, 0, [8, 8, 8, 255]);

    let result = engine.diff(&previous, &current).expect("diff result");

    assert!(result.patches.is_empty());
}

#[test]
fn changed_pixel_ratio_threshold_is_enforced() {
    let mut config = RecorderConfig::default();
    config.block_width = 4;
    config.block_height = 4;
    config.sensitivity_mode = SensitivityMode::Balanced;
    let mut engine = DiffEngine::new(&config);
    let previous = Frame::solid_rgba(8, 8, [0, 0, 0, 255]);
    let mut current = previous.clone();
    current.set_pixel(0, 0, [255, 255, 255, 255]);

    let result = engine.diff(&previous, &current).expect("diff result");

    assert!(result.patches.is_empty());
}

#[test]
fn single_frame_change_does_not_emit_immediately_when_stability_filtering_is_enabled() {
    let mut config = RecorderConfig::default();
    config.block_width = 4;
    config.block_height = 4;
    config.sensitivity_mode = SensitivityMode::Balanced;
    let mut engine = DiffEngine::new(&config);
    let previous = Frame::solid_rgba(8, 8, [0, 0, 0, 255]);
    let mut current = previous.clone();
    fill_rect(&mut current, 0, 0, 4, 4, [255, 0, 0, 255]);

    let first = engine.diff(&previous, &current).expect("diff result");
    assert!(first.patches.is_empty());
}

#[test]
fn stable_repeated_changes_are_emitted_after_stability_window_is_satisfied() {
    let mut config = RecorderConfig::default();
    config.block_width = 4;
    config.block_height = 4;
    config.sensitivity_mode = SensitivityMode::Balanced;
    let mut engine = DiffEngine::new(&config);
    let previous = Frame::solid_rgba(8, 8, [0, 0, 0, 255]);
    let mut current = previous.clone();
    fill_rect(&mut current, 0, 0, 4, 4, [255, 0, 0, 255]);

    let first = engine.diff(&previous, &current).expect("diff result");
    let second = engine.diff(&previous, &current).expect("diff result");

    assert!(first.patches.is_empty());
    assert_eq!(second.patches.len(), 1);
}

#[test]
fn moving_content_in_same_region_still_counts_as_stable_change() {
    let mut config = RecorderConfig::default();
    config.block_width = 4;
    config.block_height = 4;
    config.sensitivity_mode = SensitivityMode::Balanced;
    let mut engine = DiffEngine::new(&config);
    let previous = Frame::solid_rgba(8, 8, [0, 0, 0, 255]);

    let mut first_current = previous.clone();
    fill_rect(&mut first_current, 0, 0, 4, 4, [255, 0, 0, 255]);

    let mut second_current = previous.clone();
    fill_rect(&mut second_current, 0, 0, 4, 4, [0, 255, 0, 255]);

    let first = engine.diff(&previous, &first_current).expect("first diff");
    let second = engine
        .diff(&first_current, &second_current)
        .expect("second diff");

    assert!(first.patches.is_empty());
    assert_eq!(second.patches.len(), 1);
    assert_eq!(second.patches[0].x, 0);
    assert_eq!(second.patches[0].y, 0);
}

#[test]
fn trimmed_patch_matches_changed_pixels() {
    let mut config = RecorderConfig::default();
    config.block_width = 4;
    config.block_height = 4;
    config.sensitivity_mode = SensitivityMode::Detailed;
    let mut engine = DiffEngine::new(&config);
    let previous = Frame::solid_rgba(8, 8, [0, 0, 0, 255]);
    let mut current = previous.clone();
    fill_rect(&mut current, 1, 2, 2, 1, [255, 0, 0, 255]);

    let result = engine.diff(&previous, &current).expect("diff result");

    assert_eq!(result.patches.len(), 1);
    let patch = &result.patches[0];
    assert_eq!(patch.x, 1);
    assert_eq!(patch.y, 2);
    assert_eq!(patch.width, 2);
    assert_eq!(patch.height, 1);
    assert_eq!(patch.data.len(), 2 * 1 * 4);
    assert_eq!(&patch.data[0..4], &[255, 0, 0, 255]);
}

#[test]
fn trimmed_bbox_still_encloses_multiple_clusters() {
    let mut config = RecorderConfig::default();
    config.block_width = 4;
    config.block_height = 4;
    config.sensitivity_mode = SensitivityMode::Detailed;
    let mut engine = DiffEngine::new(&config);
    let previous = Frame::solid_rgba(8, 8, [0, 0, 0, 255]);
    let mut current = previous.clone();
    current.set_pixel(0, 0, [255, 0, 0, 255]);
    current.set_pixel(3, 3, [255, 0, 0, 255]);

    let result = engine.diff(&previous, &current).expect("diff result");

    assert_eq!(result.patches.len(), 1);
    let patch = &result.patches[0];
    assert_eq!(patch.x, 0);
    assert_eq!(patch.y, 0);
    assert_eq!(patch.width, 4);
    assert_eq!(patch.height, 4);
}

#[test]
fn partial_block_edge_trims_to_actual_pixels() {
    let mut config = RecorderConfig::default();
    config.block_width = 4;
    config.block_height = 4;
    config.sensitivity_mode = SensitivityMode::Detailed;
    let mut engine = DiffEngine::new(&config);
    let previous = Frame::solid_rgba(10, 4, [0, 0, 0, 255]);
    let mut current = previous.clone();
    current.set_pixel(9, 1, [255, 255, 255, 255]);

    let result = engine.diff(&previous, &current).expect("diff result");

    assert_eq!(result.patches.len(), 1);
    let patch = &result.patches[0];
    assert_eq!(patch.x, 9);
    assert_eq!(patch.y, 1);
    assert_eq!(patch.width, 1);
    assert_eq!(patch.height, 1);
}
