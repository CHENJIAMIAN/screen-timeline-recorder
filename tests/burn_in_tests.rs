use screen_timeline_recorder::{burn_in::burn_timestamp_overlay, frame::Frame};

const SAMPLE_TIMESTAMP_MS: u64 = 1_777_777_777_000;
const BACKGROUND_COLOR: [u8; 4] = [12, 18, 28, 255];
const TEXT_COLOR: [u8; 4] = [240, 244, 255, 255];
const SMALL_FRAME_WIDTH: usize = 120;
const SMALL_FRAME_HEIGHT: usize = 20;
const SMALL_FRAME_COLOR: [u8; 4] = [10, 20, 30, 255];
const LARGE_FRAME_WIDTH: usize = 320;
const LARGE_FRAME_HEIGHT: usize = 180;
const LARGE_FRAME_COLOR: [u8; 4] = [44, 55, 66, 255];
const EXPECTED_BOX_WIDTH: usize = 242;
const EXPECTED_BOX_HEIGHT: usize = 30;

#[test]
fn burn_timestamp_overlay_draws_visible_pixels() {
    let mut frame = Frame::solid_rgba(220, 40, [0, 0, 0, 255]);

    burn_timestamp_overlay(&mut frame, SAMPLE_TIMESTAMP_MS);

    let mut changed_pixels = 0usize;
    for y in 0..frame.height() {
        for x in 0..frame.width() {
            if frame.pixel(x, y) != [0, 0, 0, 255] {
                changed_pixels += 1;
            }
        }
    }

    assert!(changed_pixels > 0);
}

#[test]
fn burn_timestamp_overlay_respects_minimum_frame_size() {
    let mut frame = Frame::solid_rgba(SMALL_FRAME_WIDTH, SMALL_FRAME_HEIGHT, SMALL_FRAME_COLOR);
    let before = frame.as_rgba().to_vec();

    burn_timestamp_overlay(&mut frame, SAMPLE_TIMESTAMP_MS);

    assert_eq!(frame.as_rgba(), before.as_slice());
}

#[test]
fn burn_timestamp_overlay_paints_background_box_and_text() {
    let mut frame = Frame::solid_rgba(LARGE_FRAME_WIDTH, LARGE_FRAME_HEIGHT, LARGE_FRAME_COLOR);

    burn_timestamp_overlay(&mut frame, SAMPLE_TIMESTAMP_MS);

    assert_eq!(frame.pixel(0, 0), BACKGROUND_COLOR);
    assert_eq!(
        frame.pixel(EXPECTED_BOX_WIDTH - 1, EXPECTED_BOX_HEIGHT - 1),
        BACKGROUND_COLOR
    );
    assert_eq!(
        frame.pixel(frame.width() - 1, frame.height() - 1),
        LARGE_FRAME_COLOR
    );

    let mut text_pixel_found = false;
    for y in 0..EXPECTED_BOX_HEIGHT {
        for x in 0..EXPECTED_BOX_WIDTH {
            if frame.pixel(x, y) == TEXT_COLOR {
                text_pixel_found = true;
                break;
            }
        }
        if text_pixel_found {
            break;
        }
    }

    assert!(
        text_pixel_found,
        "overlay should paint at least one glyph pixel"
    );
}
