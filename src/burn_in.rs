use crate::frame::Frame;
use chrono::{Local, LocalResult, TimeZone};

const GLYPH_WIDTH: usize = 5;
const GLYPH_HEIGHT: usize = 7;
const GLYPH_SCALE: usize = 2;
const GLYPH_SPACING: usize = 2;
const PADDING_X: usize = 8;
const PADDING_Y: usize = 8;
const BACKGROUND_COLOR: [u8; 4] = [12, 18, 28, 255];
const TEXT_COLOR: [u8; 4] = [240, 244, 255, 255];
const MIN_FRAME_WIDTH: usize = 160;
const MIN_FRAME_HEIGHT: usize = 32;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OverlayBounds {
    pub x: usize,
    pub y: usize,
    pub width: usize,
    pub height: usize,
}

pub fn burn_timestamp_overlay(frame: &mut Frame, timestamp_ms: u64) {
    let Some(text) = overlay_text_for_frame(timestamp_ms, frame.width(), frame.height()) else {
        return;
    };
    let Some(bounds) = timestamp_overlay_bounds(frame, &text) else {
        return;
    };
    let glyph_pixel_width = GLYPH_WIDTH * GLYPH_SCALE;

    for y in bounds.y..(bounds.y + bounds.height) {
        for x in bounds.x..(bounds.x + bounds.width) {
            frame.set_pixel(x, y, BACKGROUND_COLOR);
        }
    }

    let mut cursor_x = PADDING_X;
    let baseline_y = PADDING_Y;
    for ch in text.chars() {
        draw_char(frame, ch, cursor_x, baseline_y);
        cursor_x += glyph_pixel_width + GLYPH_SPACING;
        if cursor_x >= frame.width() {
            break;
        }
    }
}

pub fn timestamp_overlay_bounds(frame: &Frame, text: &str) -> Option<OverlayBounds> {
    if frame.width() < MIN_FRAME_WIDTH || frame.height() < MIN_FRAME_HEIGHT {
        return None;
    }

    if text.is_empty() || frame.width() == 0 || frame.height() == 0 {
        return None;
    }

    let glyph_pixel_width = GLYPH_WIDTH * GLYPH_SCALE;
    let glyph_pixel_height = GLYPH_HEIGHT * GLYPH_SCALE;
    let text_width = text
        .chars()
        .count()
        .saturating_mul(glyph_pixel_width + GLYPH_SPACING)
        .saturating_sub(GLYPH_SPACING);

    Some(OverlayBounds {
        x: 0,
        y: 0,
        width: (text_width + PADDING_X * 2).min(frame.width()),
        height: (glyph_pixel_height + PADDING_Y * 2).min(frame.height()),
    })
}

fn draw_char(frame: &mut Frame, ch: char, origin_x: usize, origin_y: usize) {
    let glyph = glyph_rows(ch);
    for (row_index, row_bits) in glyph.iter().enumerate() {
        for col_index in 0..GLYPH_WIDTH {
            let mask = 1 << (GLYPH_WIDTH - 1 - col_index);
            if row_bits & mask == 0 {
                continue;
            }

            for dy in 0..GLYPH_SCALE {
                for dx in 0..GLYPH_SCALE {
                    let x = origin_x + col_index * GLYPH_SCALE + dx;
                    let y = origin_y + row_index * GLYPH_SCALE + dy;
                    if x < frame.width() && y < frame.height() {
                        frame.set_pixel(x, y, TEXT_COLOR);
                    }
                }
            }
        }
    }
}

pub fn format_timestamp_to_seconds(timestamp_ms: u64) -> String {
    format_timestamp_pattern(timestamp_ms, "%Y-%m-%d %H:%M:%S")
}

fn format_timestamp_pattern(timestamp_ms: u64, pattern: &str) -> String {
    let timestamp_ms = i64::try_from(timestamp_ms).ok();
    let Some(timestamp_ms) = timestamp_ms else {
        return String::new();
    };

    let local_time = match Local.timestamp_millis_opt(timestamp_ms) {
        LocalResult::Single(value) => value,
        LocalResult::Ambiguous(earliest, _) => earliest,
        LocalResult::None => return String::new(),
    };

    local_time.format(pattern).to_string()
}

fn overlay_text_for_frame(timestamp_ms: u64, frame_width: usize, frame_height: usize) -> Option<String> {
    if frame_width < MIN_FRAME_WIDTH || frame_height < MIN_FRAME_HEIGHT {
        return None;
    }

    let candidates = [
        "%Y-%m-%d %H:%M:%S",
        "%m-%d %H:%M:%S",
        "%H:%M:%S",
        "%M:%S",
    ];

    candidates
        .into_iter()
        .map(|pattern| format_timestamp_pattern(timestamp_ms, pattern))
        .find(|text| overlay_text_width(text) + PADDING_X * 2 <= frame_width)
}

fn overlay_text_width(text: &str) -> usize {
    let glyph_pixel_width = GLYPH_WIDTH * GLYPH_SCALE;
    text.chars()
        .count()
        .saturating_mul(glyph_pixel_width + GLYPH_SPACING)
        .saturating_sub(GLYPH_SPACING)
}

fn glyph_rows(ch: char) -> [u8; GLYPH_HEIGHT] {
    match ch {
        '0' => [
            0b01110, 0b10001, 0b10011, 0b10101, 0b11001, 0b10001, 0b01110,
        ],
        '1' => [
            0b00100, 0b01100, 0b00100, 0b00100, 0b00100, 0b00100, 0b01110,
        ],
        '2' => [
            0b01110, 0b10001, 0b00001, 0b00010, 0b00100, 0b01000, 0b11111,
        ],
        '3' => [
            0b11110, 0b00001, 0b00001, 0b01110, 0b00001, 0b00001, 0b11110,
        ],
        '4' => [
            0b00010, 0b00110, 0b01010, 0b10010, 0b11111, 0b00010, 0b00010,
        ],
        '5' => [
            0b11111, 0b10000, 0b10000, 0b11110, 0b00001, 0b00001, 0b11110,
        ],
        '6' => [
            0b00110, 0b01000, 0b10000, 0b11110, 0b10001, 0b10001, 0b01110,
        ],
        '7' => [
            0b11111, 0b00001, 0b00010, 0b00100, 0b01000, 0b01000, 0b01000,
        ],
        '8' => [
            0b01110, 0b10001, 0b10001, 0b01110, 0b10001, 0b10001, 0b01110,
        ],
        '9' => [
            0b01110, 0b10001, 0b10001, 0b01111, 0b00001, 0b00010, 0b11100,
        ],
        '-' => [
            0b00000, 0b00000, 0b00000, 0b11111, 0b00000, 0b00000, 0b00000,
        ],
        ':' => [
            0b00000, 0b00100, 0b00100, 0b00000, 0b00100, 0b00100, 0b00000,
        ],
        ' ' => [0b00000; GLYPH_HEIGHT],
        _ => [0b00000; GLYPH_HEIGHT],
    }
}

#[cfg(test)]
mod tests {
    use super::{format_timestamp_to_seconds, overlay_text_for_frame};
    use chrono::{Local, TimeZone};

    #[test]
    fn formats_timestamp_using_local_time() {
        let timestamp_ms = 1_777_777_777_000_i64;
        let expected = Local
            .timestamp_millis_opt(timestamp_ms)
            .single()
            .expect("timestamp should map to local time")
            .format("%Y-%m-%d %H:%M:%S")
            .to_string();

        assert_eq!(format_timestamp_to_seconds(timestamp_ms as u64), expected);
    }

    #[test]
    fn narrows_overlay_text_to_fit_smaller_frames() {
        let text = overlay_text_for_frame(1_777_777_777_000_u64, 200, 80).expect("overlay text");
        assert_eq!(text.len(), 14);
        assert!(text.contains(':'));
    }
}
