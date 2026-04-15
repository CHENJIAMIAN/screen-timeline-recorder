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

pub fn burn_timestamp_overlay(frame: &mut Frame, timestamp_ms: u64) {
    if frame.width() < MIN_FRAME_WIDTH || frame.height() < MIN_FRAME_HEIGHT {
        return;
    }

    let text = format_timestamp_to_seconds(timestamp_ms);
    if text.is_empty() || frame.width() == 0 || frame.height() == 0 {
        return;
    }

    let glyph_pixel_width = GLYPH_WIDTH * GLYPH_SCALE;
    let glyph_pixel_height = GLYPH_HEIGHT * GLYPH_SCALE;
    let text_width = text
        .chars()
        .count()
        .saturating_mul(glyph_pixel_width + GLYPH_SPACING)
        .saturating_sub(GLYPH_SPACING);
    let box_width = (text_width + PADDING_X * 2).min(frame.width());
    let box_height = (glyph_pixel_height + PADDING_Y * 2).min(frame.height());

    for y in 0..box_height {
        for x in 0..box_width {
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

fn format_timestamp_to_seconds(timestamp_ms: u64) -> String {
    let timestamp_ms = i64::try_from(timestamp_ms).ok();
    let Some(timestamp_ms) = timestamp_ms else {
        return String::new();
    };

    let local_time = match Local.timestamp_millis_opt(timestamp_ms) {
        LocalResult::Single(value) => value,
        LocalResult::Ambiguous(earliest, _) => earliest,
        LocalResult::None => return String::new(),
    };

    local_time.format("%Y-%m-%d %H:%M:%S").to_string()
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
    use super::format_timestamp_to_seconds;
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
}
