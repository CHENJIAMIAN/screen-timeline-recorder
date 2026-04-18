use std::{
    ffi::c_void,
    fmt,
    mem::size_of,
    thread,
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

use crate::{
    capture::{CaptureDimensions, CaptureSource, CapturedFrame},
    frame::Frame,
};

use windows::Win32::{
    Foundation::{GetLastError, HWND},
    Graphics::Gdi::{
        BITMAPINFO, BITMAPINFOHEADER, BitBlt, CreateCompatibleBitmap, CreateCompatibleDC,
        DIB_RGB_COLORS, DeleteDC, DeleteObject, GetDC, GetDIBits, HBITMAP, HDC, HGDIOBJ, RGBQUAD,
        ReleaseDC, SRCCOPY, SelectObject,
    },
    UI::WindowsAndMessaging::{GetSystemMetrics, SM_CXSCREEN, SM_CYSCREEN},
};

const DEFAULT_PRIMARY_DISPLAY_WIDTH: u32 = 1920;
const DEFAULT_PRIMARY_DISPLAY_HEIGHT: u32 = 1080;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BackendKind {
    PrimaryDisplayOnly,
    DesktopDuplication,
    WindowsGraphicsCapture,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WindowsCaptureError {
    UnsupportedBackend(BackendKind),
    InvalidDimensions { width: u32, height: u32 },
    InvalidFrameBuffer { expected: usize, actual: usize },
    WinApiError { context: &'static str, code: u32 },
}

impl fmt::Display for WindowsCaptureError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnsupportedBackend(kind) => write!(f, "unsupported backend: {kind:?}"),
            Self::InvalidDimensions { width, height } => {
                write!(f, "invalid capture dimensions: {width}x{height}")
            }
            Self::InvalidFrameBuffer { expected, actual } => write!(
                f,
                "invalid frame buffer size: expected {expected} bytes, got {actual} bytes"
            ),
            Self::WinApiError { context, code } => {
                write!(f, "windows api error in {context}: {code}")
            }
        }
    }
}

impl std::error::Error for WindowsCaptureError {}

#[derive(Debug)]
pub struct WindowsCapture {
    backend: BackendKind,
    dimensions: CaptureDimensions,
    sample_interval: Duration,
    last_capture_at: Option<Instant>,
}

impl WindowsCapture {
    pub fn new(backend: BackendKind) -> Result<Self, WindowsCaptureError> {
        Self::with_scale(backend, 1_000, 1.0)
    }

    pub fn with_interval(
        backend: BackendKind,
        sample_interval_ms: u64,
    ) -> Result<Self, WindowsCaptureError> {
        Self::with_scale(backend, sample_interval_ms, 1.0)
    }

    pub fn with_scale(
        backend: BackendKind,
        sample_interval_ms: u64,
        working_scale: f32,
    ) -> Result<Self, WindowsCaptureError> {
        match backend {
            BackendKind::PrimaryDisplayOnly => {
                let dimensions = primary_display_dimensions(working_scale);
                Ok(Self {
                    backend,
                    dimensions,
                    sample_interval: Duration::from_millis(sample_interval_ms),
                    last_capture_at: None,
                })
            }
            _ => Err(WindowsCaptureError::UnsupportedBackend(backend)),
        }
    }

    pub fn backend_kind(&self) -> BackendKind {
        self.backend
    }
}

impl CaptureSource for WindowsCapture {
    fn dimensions(&self) -> CaptureDimensions {
        self.dimensions
    }

    fn next_frame(&mut self) -> Option<CapturedFrame> {
        if let Some(last_capture_at) = self.last_capture_at {
            let elapsed = last_capture_at.elapsed();
            if elapsed < self.sample_interval {
                thread::sleep(self.sample_interval - elapsed);
            }
        }

        let timestamp_ms = current_timestamp_ms();
        let frame = capture_primary_display_frame(
            self.dimensions.display_width,
            self.dimensions.display_height,
        )
        .ok()?
        .into_frame()
        .ok()?
        .resize_for_capture(
            self.dimensions.working_width as usize,
            self.dimensions.working_height as usize,
        );
        self.last_capture_at = Some(Instant::now());

        Some(CapturedFrame {
            timestamp_ms,
            frame,
        })
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct WindowsFrame {
    width: u32,
    height: u32,
    rgba: Vec<u8>,
}

impl WindowsFrame {
    pub fn new(width: u32, height: u32, rgba: Vec<u8>) -> Self {
        Self {
            width,
            height,
            rgba,
        }
    }

    pub fn into_frame(self) -> Result<Frame, WindowsCaptureError> {
        let expected = self.width as usize * self.height as usize * 4;
        if self.rgba.len() != expected {
            return Err(WindowsCaptureError::InvalidFrameBuffer {
                expected,
                actual: self.rgba.len(),
            });
        }

        Ok(Frame::from_rgba(
            self.width as usize,
            self.height as usize,
            self.rgba,
        ))
    }
}

fn primary_display_dimensions(working_scale: f32) -> CaptureDimensions {
    let (display_width, display_height) = primary_display_size();
    let working_width = ((display_width as f32) * working_scale)
        .round()
        .clamp(1.0, display_width as f32) as u32;
    let working_height = ((display_height as f32) * working_scale)
        .round()
        .clamp(1.0, display_height as f32) as u32;
    CaptureDimensions {
        display_width,
        display_height,
        working_width,
        working_height,
    }
}

fn primary_display_size() -> (u32, u32) {
    let width = unsafe { GetSystemMetrics(SM_CXSCREEN) };
    let height = unsafe { GetSystemMetrics(SM_CYSCREEN) };
    let width = if width > 0 {
        width as u32
    } else {
        DEFAULT_PRIMARY_DISPLAY_WIDTH
    };
    let height = if height > 0 {
        height as u32
    } else {
        DEFAULT_PRIMARY_DISPLAY_HEIGHT
    };
    (width, height)
}

fn capture_primary_display_frame(
    width: u32,
    height: u32,
) -> Result<WindowsFrame, WindowsCaptureError> {
    if width == 0 || height == 0 {
        return Err(WindowsCaptureError::InvalidDimensions { width, height });
    }

    let width_i32 = width as i32;
    let height_i32 = height as i32;
    let hwnd = HWND(0);

    unsafe {
        let screen_dc = GetDC(hwnd);
        if screen_dc.0 == 0 {
            return Err(last_error("GetDC"));
        }
        let _screen_dc = ScreenDc {
            hwnd,
            hdc: screen_dc,
        };

        let mem_dc = CreateCompatibleDC(screen_dc);
        if mem_dc.0 == 0 {
            return Err(last_error("CreateCompatibleDC"));
        }
        let _mem_dc = MemDc { hdc: mem_dc };

        let bitmap = CreateCompatibleBitmap(screen_dc, width_i32, height_i32);
        if bitmap.0 == 0 {
            return Err(last_error("CreateCompatibleBitmap"));
        }
        let bitmap = GdiBitmap { bitmap };

        let old_obj = SelectObject(mem_dc, HGDIOBJ(bitmap.bitmap.0));
        if old_obj.0 == 0 {
            return Err(last_error("SelectObject"));
        }
        let _select_guard = SelectGuard {
            dc: mem_dc,
            old: old_obj,
        };

        if BitBlt(
            mem_dc, 0, 0, width_i32, height_i32, screen_dc, 0, 0, SRCCOPY,
        )
        .is_err()
        {
            return Err(last_error("BitBlt"));
        }

        let mut buffer = vec![0u8; width as usize * height as usize * 4];
        let mut bmi = BITMAPINFO {
            bmiHeader: BITMAPINFOHEADER {
                biSize: size_of::<BITMAPINFOHEADER>() as u32,
                biWidth: width_i32,
                biHeight: -height_i32,
                biPlanes: 1,
                biBitCount: 32,
                biCompression: 0,
                biSizeImage: 0,
                biXPelsPerMeter: 0,
                biYPelsPerMeter: 0,
                biClrUsed: 0,
                biClrImportant: 0,
            },
            bmiColors: [RGBQUAD::default()],
        };

        let lines = GetDIBits(
            mem_dc,
            bitmap.bitmap,
            0,
            height as u32,
            Some(buffer.as_mut_ptr() as *mut c_void),
            &mut bmi,
            DIB_RGB_COLORS,
        );
        if lines == 0 {
            return Err(last_error("GetDIBits"));
        }

        for pixel in buffer.chunks_exact_mut(4) {
            pixel.swap(0, 2);
        }

        Ok(WindowsFrame::new(width, height, buffer))
    }
}

fn current_timestamp_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

fn last_error(context: &'static str) -> WindowsCaptureError {
    WindowsCaptureError::WinApiError {
        context,
        code: unsafe { GetLastError().0 },
    }
}

struct ScreenDc {
    hwnd: HWND,
    hdc: HDC,
}

impl Drop for ScreenDc {
    fn drop(&mut self) {
        unsafe {
            if self.hdc.0 != 0 {
                let _ = ReleaseDC(self.hwnd, self.hdc);
            }
        }
    }
}

struct MemDc {
    hdc: HDC,
}

impl Drop for MemDc {
    fn drop(&mut self) {
        unsafe {
            if self.hdc.0 != 0 {
                let _ = DeleteDC(self.hdc);
            }
        }
    }
}

struct GdiBitmap {
    bitmap: HBITMAP,
}

impl Drop for GdiBitmap {
    fn drop(&mut self) {
        unsafe {
            if self.bitmap.0 != 0 {
                let _ = DeleteObject(HGDIOBJ(self.bitmap.0));
            }
        }
    }
}

struct SelectGuard {
    dc: HDC,
    old: HGDIOBJ,
}

impl Drop for SelectGuard {
    fn drop(&mut self) {
        unsafe {
            if self.old.0 != 0 {
                let _ = SelectObject(self.dc, self.old);
            }
        }
    }
}
