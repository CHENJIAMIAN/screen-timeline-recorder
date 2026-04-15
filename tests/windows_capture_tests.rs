#![cfg(target_os = "windows")]

use screen_timeline_recorder::capture::CaptureSource;
use screen_timeline_recorder::capture::windows::{
    BackendKind, WindowsCapture, WindowsCaptureError, WindowsFrame,
};

#[test]
fn primary_display_only_backend_is_selected() {
    let capture =
        WindowsCapture::new(BackendKind::PrimaryDisplayOnly).expect("primary display capture");

    assert_eq!(capture.backend_kind(), BackendKind::PrimaryDisplayOnly);
    let dimensions = capture.dimensions();
    assert!(dimensions.display_width > 0);
    assert!(dimensions.display_height > 0);
    assert_eq!(dimensions.display_width, dimensions.working_width);
    assert_eq!(dimensions.display_height, dimensions.working_height);
}

#[test]
fn unsupported_backend_returns_error() {
    let err = WindowsCapture::new(BackendKind::DesktopDuplication)
        .expect_err("unsupported backend should error");

    assert_eq!(
        err,
        WindowsCaptureError::UnsupportedBackend(BackendKind::DesktopDuplication)
    );
}

#[test]
fn windows_frame_converts_into_frame() {
    let rgba = vec![10, 20, 30, 255, 40, 50, 60, 255];
    let frame = WindowsFrame::new(2, 1, rgba.clone())
        .into_frame()
        .expect("frame conversion");

    assert_eq!(frame.width(), 2);
    assert_eq!(frame.height(), 1);
    assert_eq!(frame.as_rgba(), rgba.as_slice());
}
