#[derive(Debug, Clone, PartialEq)]
pub struct Frame {
    width: usize,
    height: usize,
    rgba: Vec<u8>,
}

impl Frame {
    pub fn from_rgba(width: usize, height: usize, rgba: Vec<u8>) -> Self {
        assert_eq!(rgba.len(), width * height * 4, "rgba buffer size mismatch");
        Self {
            width,
            height,
            rgba,
        }
    }

    pub fn solid_rgba(width: usize, height: usize, rgba: [u8; 4]) -> Self {
        let mut buffer = vec![0u8; width * height * 4];
        for y in 0..height {
            for x in 0..width {
                let idx = (y * width + x) * 4;
                buffer[idx..idx + 4].copy_from_slice(&rgba);
            }
        }

        Self {
            width,
            height,
            rgba: buffer,
        }
    }

    pub fn width(&self) -> usize {
        self.width
    }

    pub fn height(&self) -> usize {
        self.height
    }

    pub fn set_pixel(&mut self, x: usize, y: usize, rgba: [u8; 4]) {
        let idx = self.pixel_index(x, y);
        self.rgba[idx..idx + 4].copy_from_slice(&rgba);
    }

    pub fn pixel(&self, x: usize, y: usize) -> [u8; 4] {
        let idx = self.pixel_index(x, y);
        [
            self.rgba[idx],
            self.rgba[idx + 1],
            self.rgba[idx + 2],
            self.rgba[idx + 3],
        ]
    }

    pub fn copy_region_rgba(
        &self,
        start_x: usize,
        start_y: usize,
        width: usize,
        height: usize,
    ) -> Vec<u8> {
        let mut region = Vec::with_capacity(width * height * 4);
        for y in start_y..(start_y + height) {
            for x in start_x..(start_x + width) {
                let idx = self.pixel_index(x, y);
                region.extend_from_slice(&self.rgba[idx..idx + 4]);
            }
        }
        region
    }

    pub fn as_rgba(&self) -> &[u8] {
        &self.rgba
    }

    pub fn sampled_difference_ratio(
        &self,
        other: &Self,
        sample_columns: usize,
        sample_rows: usize,
    ) -> f32 {
        assert_eq!(self.width, other.width, "frame widths must match");
        assert_eq!(self.height, other.height, "frame heights must match");
        assert!(
            sample_columns > 0 && sample_rows > 0,
            "sample grid must be greater than zero"
        );

        let sample_columns = sample_columns.min(self.width).max(1);
        let sample_rows = sample_rows.min(self.height).max(1);
        let mut total_delta = 0.0f32;

        for row in 0..sample_rows {
            let y = sample_coordinate(self.height, sample_rows, row);
            for column in 0..sample_columns {
                let x = sample_coordinate(self.width, sample_columns, column);
                total_delta += normalized_pixel_difference(self.pixel(x, y), other.pixel(x, y));
            }
        }

        total_delta / (sample_columns * sample_rows) as f32
    }

    pub fn resize_for_capture(&self, target_width: usize, target_height: usize) -> Self {
        assert!(target_width > 0, "target_width must be greater than zero");
        assert!(target_height > 0, "target_height must be greater than zero");

        if target_width >= self.width && target_height >= self.height {
            return self.resize_nearest(target_width, target_height);
        }

        self.resize_box_average(target_width, target_height)
    }

    fn resize_nearest(&self, target_width: usize, target_height: usize) -> Self {
        let mut rgba = vec![0u8; target_width * target_height * 4];
        for target_y in 0..target_height {
            let source_y = target_y * self.height / target_height;
            for target_x in 0..target_width {
                let source_x = target_x * self.width / target_width;
                let source_idx = self.pixel_index(source_x, source_y);
                let target_idx = (target_y * target_width + target_x) * 4;
                rgba[target_idx..target_idx + 4]
                    .copy_from_slice(&self.rgba[source_idx..source_idx + 4]);
            }
        }

        Self {
            width: target_width,
            height: target_height,
            rgba,
        }
    }

    fn resize_box_average(&self, target_width: usize, target_height: usize) -> Self {
        let mut rgba = vec![0u8; target_width * target_height * 4];
        for target_y in 0..target_height {
            let source_y0 = target_y * self.height / target_height;
            let source_y1 = ((target_y + 1) * self.height).div_ceil(target_height);
            for target_x in 0..target_width {
                let source_x0 = target_x * self.width / target_width;
                let source_x1 = ((target_x + 1) * self.width).div_ceil(target_width);
                let target_idx = (target_y * target_width + target_x) * 4;
                let mut sums = [0u32; 4];
                let mut count = 0u32;
                for source_y in source_y0..source_y1 {
                    for source_x in source_x0..source_x1 {
                        let source_idx = self.pixel_index(source_x, source_y);
                        sums[0] += self.rgba[source_idx] as u32;
                        sums[1] += self.rgba[source_idx + 1] as u32;
                        sums[2] += self.rgba[source_idx + 2] as u32;
                        sums[3] += self.rgba[source_idx + 3] as u32;
                        count += 1;
                    }
                }
                rgba[target_idx] = ((sums[0] + count / 2) / count) as u8;
                rgba[target_idx + 1] = ((sums[1] + count / 2) / count) as u8;
                rgba[target_idx + 2] = ((sums[2] + count / 2) / count) as u8;
                rgba[target_idx + 3] = ((sums[3] + count / 2) / count) as u8;
            }
        }

        Self {
            width: target_width,
            height: target_height,
            rgba,
        }
    }

    fn pixel_index(&self, x: usize, y: usize) -> usize {
        assert!(x < self.width, "x out of bounds");
        assert!(y < self.height, "y out of bounds");
        (y * self.width + x) * 4
    }
}

fn sample_coordinate(size: usize, samples: usize, index: usize) -> usize {
    ((index * 2 + 1) * size) / (samples * 2)
}

fn normalized_pixel_difference(previous: [u8; 4], current: [u8; 4]) -> f32 {
    let red = (previous[0] as f32 - current[0] as f32).abs() / 255.0;
    let green = (previous[1] as f32 - current[1] as f32).abs() / 255.0;
    let blue = (previous[2] as f32 - current[2] as f32).abs() / 255.0;
    (red + green + blue) / 3.0
}
