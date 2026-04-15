use std::collections::VecDeque;

use super::{CaptureDimensions, CaptureSource, CapturedFrame};

#[derive(Debug)]
pub struct MockCapture {
    dimensions: CaptureDimensions,
    frames: VecDeque<CapturedFrame>,
}

impl MockCapture {
    pub fn new(dimensions: CaptureDimensions, frames: Vec<CapturedFrame>) -> Self {
        Self {
            dimensions,
            frames: frames.into(),
        }
    }
}

impl CaptureSource for MockCapture {
    fn dimensions(&self) -> CaptureDimensions {
        self.dimensions
    }

    fn next_frame(&mut self) -> Option<CapturedFrame> {
        self.frames.pop_front()
    }
}
