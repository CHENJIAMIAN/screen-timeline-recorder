use screen_timeline_recorder::frame::Frame;

#[test]
fn resize_nearest_downscales_rgba_frames() {
    let mut frame = Frame::solid_rgba(4, 4, [0, 0, 0, 255]);
    frame.set_pixel(0, 0, [255, 0, 0, 255]);
    frame.set_pixel(2, 0, [0, 255, 0, 255]);
    frame.set_pixel(0, 2, [0, 0, 255, 255]);
    frame.set_pixel(2, 2, [255, 255, 0, 255]);

    let resized = frame.resize_nearest(2, 2);

    assert_eq!(resized.width(), 2);
    assert_eq!(resized.height(), 2);
    assert_eq!(resized.pixel(0, 0), [255, 0, 0, 255]);
    assert_eq!(resized.pixel(1, 0), [0, 255, 0, 255]);
    assert_eq!(resized.pixel(0, 1), [0, 0, 255, 255]);
    assert_eq!(resized.pixel(1, 1), [255, 255, 0, 255]);
}

#[test]
fn sampled_difference_ratio_is_zero_for_identical_frames() {
    let frame = Frame::solid_rgba(8, 8, [20, 30, 40, 255]);

    let ratio = frame.sampled_difference_ratio(&frame, 4, 4);

    assert_eq!(ratio, 0.0);
}

#[test]
fn sampled_difference_ratio_detects_changes() {
    let base = Frame::solid_rgba(8, 8, [0, 0, 0, 255]);
    let mut changed = base.clone();
    changed.set_pixel(3, 3, [255, 255, 255, 255]);

    let ratio = base.sampled_difference_ratio(&changed, 8, 8);

    assert!(ratio > 0.0);
}

#[test]
fn sampled_difference_ratio_handles_non_divisible_sample_grids() {
    let base = Frame::solid_rgba(7, 5, [0, 0, 0, 255]);
    let mut changed = base.clone();
    changed.set_pixel(6, 4, [255, 255, 255, 255]);

    let ratio = base.sampled_difference_ratio(&changed, 3, 2);

    assert!(ratio >= 0.0);
    assert!(ratio <= 1.0);
}
