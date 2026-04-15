use crate::frame::Frame;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CaptureDimensions {
    pub display_width: u32,
    pub display_height: u32,
    pub working_width: u32,
    pub working_height: u32,
}

#[derive(Debug, Clone, PartialEq)]
pub struct CapturedFrame {
    pub timestamp_ms: u64,
    pub frame: Frame,
}

pub trait CaptureSource {
    fn dimensions(&self) -> CaptureDimensions;
    fn next_frame(&mut self) -> Option<CapturedFrame>;
}

pub mod mock;
#[cfg(target_os = "windows")]
pub mod windows;
