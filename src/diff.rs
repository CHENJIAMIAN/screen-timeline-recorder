use std::collections::HashMap;

use crate::config::RecorderConfig;
use crate::frame::Frame;

#[derive(Debug, Clone, PartialEq)]
pub struct PatchRegion {
    pub x: u32,
    pub y: u32,
    pub width: u32,
    pub height: u32,
    pub data: Vec<u8>,
}

impl PatchRegion {
    pub fn matches_region(&self, other: &PatchRegion) -> bool {
        self.x == other.x
            && self.y == other.y
            && self.width == other.width
            && self.height == other.height
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct DiffResult {
    pub patches: Vec<PatchRegion>,
    pub precheck_skipped: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DiffError {
    DimensionMismatch,
}

impl std::fmt::Display for DiffError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::DimensionMismatch => write!(f, "frame dimensions do not match"),
        }
    }
}

impl std::error::Error for DiffError {}

#[derive(Debug)]
pub struct DiffEngine {
    block_width: usize,
    block_height: usize,
    precheck_threshold: f32,
    block_difference_threshold: f32,
    changed_pixel_ratio_threshold: f32,
    stability_window: u32,
    stability_counts: HashMap<RegionKey, u32>,
}

impl DiffEngine {
    pub fn new(config: &RecorderConfig) -> Self {
        let thresholds = config.thresholds();
        Self {
            block_width: config.block_width as usize,
            block_height: config.block_height as usize,
            precheck_threshold: thresholds.precheck_threshold,
            block_difference_threshold: thresholds.block_difference_threshold,
            changed_pixel_ratio_threshold: thresholds.changed_pixel_ratio_threshold,
            stability_window: thresholds.stability_window,
            stability_counts: HashMap::new(),
        }
    }

    pub fn diff(&mut self, previous: &Frame, current: &Frame) -> Result<DiffResult, DiffError> {
        if previous.width() != current.width() || previous.height() != current.height() {
            return Err(DiffError::DimensionMismatch);
        }

        let precheck_score = normalized_frame_difference(previous, current);
        if precheck_score < self.precheck_threshold {
            return Ok(DiffResult {
                patches: Vec::new(),
                precheck_skipped: true,
            });
        }

        let mut next_counts = HashMap::new();
        let mut patches = Vec::new();

        let width = previous.width();
        let height = previous.height();

        for block_y in (0..height).step_by(self.block_height) {
            for block_x in (0..width).step_by(self.block_width) {
                let region_width = self.block_width.min(width - block_x);
                let region_height = self.block_height.min(height - block_y);

                let Some(trimmed_region) = self.changed_region_bounds(
                    previous,
                    current,
                    block_x,
                    block_y,
                    region_width,
                    region_height,
                ) else {
                    continue;
                };

                let key = RegionKey::new(block_x, block_y, region_width, region_height);
                let count = self.stability_counts.get(&key).copied().unwrap_or(0) + 1;
                next_counts.insert(key, count);

                if count >= self.stability_window {
                    patches.push(PatchRegion {
                        x: trimmed_region.x as u32,
                        y: trimmed_region.y as u32,
                        width: trimmed_region.width as u32,
                        height: trimmed_region.height as u32,
                        data: current.copy_region_rgba(
                            trimmed_region.x,
                            trimmed_region.y,
                            trimmed_region.width,
                            trimmed_region.height,
                        ),
                    });
                }
            }
        }

        self.stability_counts = next_counts;

        Ok(DiffResult {
            patches,
            precheck_skipped: false,
        })
    }

    fn changed_region_bounds(
        &self,
        previous: &Frame,
        current: &Frame,
        start_x: usize,
        start_y: usize,
        width: usize,
        height: usize,
    ) -> Option<ChangedRegion> {
        let mut changed_pixels = 0usize;
        let total_pixels = width * height;
        let mut min_x = usize::MAX;
        let mut min_y = usize::MAX;
        let mut max_x = 0usize;
        let mut max_y = 0usize;

        for y in start_y..(start_y + height) {
            for x in start_x..(start_x + width) {
                let delta = normalized_pixel_difference(previous.pixel(x, y), current.pixel(x, y));
                if delta >= self.block_difference_threshold {
                    changed_pixels += 1;
                    min_x = min_x.min(x);
                    min_y = min_y.min(y);
                    max_x = max_x.max(x);
                    max_y = max_y.max(y);
                }
            }
        }

        if changed_pixels == 0 {
            return None;
        }

        let ratio = changed_pixels as f32 / total_pixels as f32;
        if ratio < self.changed_pixel_ratio_threshold {
            return None;
        }

        Some(ChangedRegion {
            x: min_x,
            y: min_y,
            width: max_x - min_x + 1,
            height: max_y - min_y + 1,
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct ChangedRegion {
    x: usize,
    y: usize,
    width: usize,
    height: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct RegionKey {
    x: usize,
    y: usize,
    width: usize,
    height: usize,
}

impl RegionKey {
    fn new(x: usize, y: usize, width: usize, height: usize) -> Self {
        Self {
            x,
            y,
            width,
            height,
        }
    }
}

fn normalized_frame_difference(previous: &Frame, current: &Frame) -> f32 {
    let mut total_delta = 0.0f32;
    let total_pixels = previous.width() * previous.height();

    for y in 0..previous.height() {
        for x in 0..previous.width() {
            total_delta += normalized_pixel_difference(previous.pixel(x, y), current.pixel(x, y));
        }
    }

    total_delta / total_pixels as f32
}

fn normalized_pixel_difference(previous: [u8; 4], current: [u8; 4]) -> f32 {
    let red = (previous[0] as f32 - current[0] as f32).abs() / 255.0;
    let green = (previous[1] as f32 - current[1] as f32).abs() / 255.0;
    let blue = (previous[2] as f32 - current[2] as f32).abs() / 255.0;
    (red + green + blue) / 3.0
}
