use crate::{
    config::{RecorderConfig, ViewerLanguage},
    session_control::{pause_session, read_status, resume_session, stop_session},
    viewer_api::get_sessions,
    viewer_server::ViewerServer,
};

#[cfg(target_os = "windows")]
use std::{
    net::TcpListener,
    os::windows::process::CommandExt,
    path::{Path, PathBuf},
    process::Command,
    thread,
    time::{Duration, SystemTime, UNIX_EPOCH},
};

#[cfg(target_os = "windows")]
use tauri::{
    Manager, Url, WebviewUrl, WebviewWindowBuilder, WindowEvent,
    image::Image,
    menu::{Menu, MenuItem, PredefinedMenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
};

#[cfg(target_os = "windows")]
use tauri_plugin_global_shortcut::{Builder as GlobalShortcutBuilder, GlobalShortcutExt};

#[cfg(target_os = "windows")]
use windows::Win32::Globalization::GetUserDefaultUILanguage;

#[cfg(target_os = "windows")]
const CREATE_NO_WINDOW: u32 = 0x0800_0000;
#[cfg(target_os = "windows")]
const MAIN_WINDOW_LABEL: &str = "main";
#[cfg(target_os = "windows")]
const TRAY_ID: &str = "main-tray";
#[cfg(target_os = "windows")]
const MENU_OPEN_UI: &str = "open-ui";
#[cfg(target_os = "windows")]
const MENU_START_RECORDING: &str = "start-recording";
#[cfg(target_os = "windows")]
const MENU_PAUSE_RESUME: &str = "pause-resume";
#[cfg(target_os = "windows")]
const MENU_STOP_RECORDING: &str = "stop-recording";
#[cfg(target_os = "windows")]
const MENU_QUIT: &str = "quit";
#[cfg(target_os = "windows")]
const SHORTCUT_OPEN_UI: &str = "Ctrl+Alt+Shift+O";
#[cfg(target_os = "windows")]
const SHORTCUT_START_RECORDING: &str = "Ctrl+Alt+Shift+R";
#[cfg(target_os = "windows")]
const SHORTCUT_PAUSE_RESUME: &str = "Ctrl+Alt+Shift+P";
#[cfg(target_os = "windows")]
const SHORTCUT_STOP_RECORDING: &str = "Ctrl+Alt+Shift+S";

#[cfg(target_os = "windows")]
#[derive(Clone)]
struct DesktopState {
    output_dir: PathBuf,
    language: DesktopLanguage,
}

#[cfg(target_os = "windows")]
#[derive(Clone, Copy)]
enum DesktopLanguage {
    En,
    Zh,
}

#[cfg(target_os = "windows")]
pub fn run_desktop(
    config: RecorderConfig,
    background: bool,
    autorun_record: bool,
) -> Result<(), String> {
    let bind_addr = allocate_bind_addr()?;
    let session_id = latest_session_id_or_placeholder(&config);
    let server = ViewerServer::new(config.output_dir.clone(), session_id);
    let server_bind_addr = bind_addr.clone();
    let _server_thread = thread::spawn(move || {
        let _ = server.serve(&server_bind_addr);
    });

    let viewer_url = format!("http://{bind_addr}/");
    let parsed_viewer_url = Url::parse(&viewer_url).map_err(|err| err.to_string())?;
    let webview_url = WebviewUrl::External(parsed_viewer_url);
    let language = resolve_desktop_language(config.viewer_language);
    let state = DesktopState {
        output_dir: config.output_dir.clone(),
        language,
    };

    tauri::Builder::default()
        .plugin(GlobalShortcutBuilder::new().build())
        .manage(state.clone())
        .on_window_event(|window, event| {
            if let WindowEvent::CloseRequested { api, .. } = event {
                api.prevent_close();
                let _ = window.hide();
            }
        })
        .setup(move |app| {
            let handle = app.handle().clone();
            let window = WebviewWindowBuilder::new(app, MAIN_WINDOW_LABEL, webview_url.clone())
                .title(desktop_text(language).app_name)
                .inner_size(1400.0, 940.0)
                .min_inner_size(1024.0, 720.0)
                .visible(!background)
                .build()?;

            if background {
                let _ = window.hide();
            }

            build_tray(&handle)?;
            register_shortcuts(&handle)?;

            if autorun_record && active_recording_session_id(&state.output_dir).is_none() {
                let _ = start_new_recording(&state.output_dir);
            }

            Ok(())
        })
        .run(tauri::generate_context!())
        .map_err(|err| err.to_string())
}

#[cfg(target_os = "windows")]
fn build_tray<R: tauri::Runtime>(app: &tauri::AppHandle<R>) -> tauri::Result<()> {
    let language = {
        let state = app.state::<DesktopState>();
        state.language
    };
    let text = desktop_text(language);

    let menu = Menu::new(app)?;
    let open_ui_item = MenuItem::with_id(app, MENU_OPEN_UI, text.open_main_ui, true, None::<&str>)?;
    let start_item = MenuItem::with_id(
        app,
        MENU_START_RECORDING,
        text.start_new_recording,
        true,
        None::<&str>,
    )?;
    let pause_resume_item = MenuItem::with_id(
        app,
        MENU_PAUSE_RESUME,
        text.pause_resume,
        true,
        None::<&str>,
    )?;
    let stop_item = MenuItem::with_id(
        app,
        MENU_STOP_RECORDING,
        text.stop_current_recording,
        true,
        None::<&str>,
    )?;
    let quit_item = MenuItem::with_id(app, MENU_QUIT, text.quit, true, None::<&str>)?;
    menu.append(&open_ui_item)?;
    menu.append(&PredefinedMenuItem::separator(app)?)?;
    menu.append(&start_item)?;
    menu.append(&pause_resume_item)?;
    menu.append(&stop_item)?;
    menu.append(&PredefinedMenuItem::separator(app)?)?;
    menu.append(&quit_item)?;

    let icon = Image::from_path(resolve_icon_path())?;
    TrayIconBuilder::with_id(TRAY_ID)
        .icon(icon)
        .tooltip(text.app_name)
        .show_menu_on_left_click(false)
        .menu(&menu)
        .on_menu_event(|app, event| {
            let state = app.state::<DesktopState>();
            match event.id().as_ref() {
                MENU_OPEN_UI => {
                    let _ = open_main_window(app);
                }
                MENU_START_RECORDING => {
                    let _ = start_new_recording(&state.output_dir);
                }
                MENU_PAUSE_RESUME => {
                    let _ = toggle_pause_resume(&state.output_dir);
                }
                MENU_STOP_RECORDING => {
                    let _ = stop_active_recording(&state.output_dir);
                }
                MENU_QUIT => {
                    app.exit(0);
                }
                _ => {}
            }
        })
        .on_tray_icon_event(|tray, event| {
            if matches!(
                event,
                TrayIconEvent::Click {
                    button: MouseButton::Left,
                    button_state: MouseButtonState::Up,
                    ..
                }
            ) || matches!(
                event,
                TrayIconEvent::DoubleClick {
                    button: MouseButton::Left,
                    ..
                }
            ) {
                let _ = open_main_window(&tray.app_handle());
            }
        })
        .build(app)?;

    spawn_tray_status_updater(
        app.clone(),
        language,
        open_ui_item,
        start_item,
        pause_resume_item,
        stop_item,
    );
    Ok(())
}

#[cfg(target_os = "windows")]
fn register_shortcuts<R: tauri::Runtime>(app: &tauri::AppHandle<R>) -> Result<(), String> {
    app.global_shortcut()
        .on_shortcut(SHORTCUT_OPEN_UI, move |app, _, event| {
            if event.state == tauri_plugin_global_shortcut::ShortcutState::Pressed {
                let _ = open_main_window(app);
            }
        })
        .map_err(|err| err.to_string())?;
    app.global_shortcut()
        .on_shortcut(SHORTCUT_START_RECORDING, move |app, _, event| {
            if event.state == tauri_plugin_global_shortcut::ShortcutState::Pressed {
                let state = app.state::<DesktopState>();
                let _ = start_new_recording(&state.output_dir);
            }
        })
        .map_err(|err| err.to_string())?;
    app.global_shortcut()
        .on_shortcut(SHORTCUT_PAUSE_RESUME, move |app, _, event| {
            if event.state == tauri_plugin_global_shortcut::ShortcutState::Pressed {
                let state = app.state::<DesktopState>();
                let _ = toggle_pause_resume(&state.output_dir);
            }
        })
        .map_err(|err| err.to_string())?;
    app.global_shortcut()
        .on_shortcut(SHORTCUT_STOP_RECORDING, move |app, _, event| {
            if event.state == tauri_plugin_global_shortcut::ShortcutState::Pressed {
                let state = app.state::<DesktopState>();
                let _ = stop_active_recording(&state.output_dir);
            }
        })
        .map_err(|err| err.to_string())?;
    Ok(())
}

#[cfg(target_os = "windows")]
fn open_main_window<R: tauri::Runtime>(app: &tauri::AppHandle<R>) -> Result<(), String> {
    let window = app
        .get_webview_window(MAIN_WINDOW_LABEL)
        .ok_or_else(|| "desktop window not found".to_string())?;
    let _ = window.show();
    let _ = window.unminimize();
    let _ = window.set_focus();
    Ok(())
}

#[cfg(target_os = "windows")]
fn start_new_recording(output_dir: &Path) -> Result<String, String> {
    if let Some(active_session_id) = active_recording_session_id(output_dir) {
        return Err(format!(
            "recording is already active: {active_session_id}. Stop it before starting a new one."
        ));
    }

    let exe_path = std::env::current_exe().map_err(|err| err.to_string())?;
    let session_id = generated_session_id();
    let mut command = Command::new(exe_path);
    command
        .creation_flags(CREATE_NO_WINDOW)
        .arg("record-video")
        .arg("--session-id")
        .arg(&session_id)
        .arg("--output-dir")
        .arg(output_dir);
    command.spawn().map_err(|err| err.to_string())?;
    Ok(session_id)
}

#[cfg(target_os = "windows")]
fn toggle_pause_resume(output_dir: &Path) -> Result<(), String> {
    let session_id = active_recording_session_id(output_dir)
        .ok_or_else(|| "no active recording session".to_string())?;
    let status = read_status(output_dir, &session_id).map_err(|err| err.to_string())?;
    if matches!(status.state, crate::session::SessionState::Paused) {
        resume_session(output_dir, &session_id).map_err(|err| err.to_string())
    } else {
        pause_session(output_dir, &session_id).map_err(|err| err.to_string())
    }
}

#[cfg(target_os = "windows")]
fn stop_active_recording(output_dir: &Path) -> Result<(), String> {
    let session_id = active_recording_session_id(output_dir)
        .ok_or_else(|| "no active recording session".to_string())?;
    stop_session(output_dir, &session_id).map_err(|err| err.to_string())
}

#[cfg(target_os = "windows")]
fn active_recording_session_id(output_dir: &Path) -> Option<String> {
    get_sessions(output_dir)
        .ok()?
        .into_iter()
        .find(|session| {
            session
                .status
                .as_ref()
                .is_some_and(|status| status.recording)
        })
        .map(|session| session.session_id)
}

#[cfg(target_os = "windows")]
fn spawn_tray_status_updater<R: tauri::Runtime>(
    app: tauri::AppHandle<R>,
    language: DesktopLanguage,
    open_ui_item: MenuItem<R>,
    start_item: MenuItem<R>,
    pause_resume_item: MenuItem<R>,
    stop_item: MenuItem<R>,
) {
    thread::spawn(move || {
        loop {
            let output_dir = {
                let state = app.state::<DesktopState>();
                state.output_dir.clone()
            };

            let text = desktop_text(language);
            let status = recording_status_summary(&output_dir, language);
            if let Some(tray) = app.tray_by_id(TRAY_ID) {
                let _ = tray.set_tooltip(Some(status.tooltip.clone()));
            }
            let _ =
                open_ui_item.set_text(format!("{} ({})", text.open_main_ui, status.short_label));
            let _ = start_item.set_enabled(!status.recording);
            let _ = start_item.set_text(if status.recording {
                text.start_new_recording_busy
            } else {
                text.start_new_recording
            });
            let _ = pause_resume_item.set_enabled(status.recording);
            let _ = pause_resume_item.set_text(status.pause_resume_label);
            let _ = stop_item.set_enabled(status.recording);
            let _ = stop_item.set_text(status.stop_label);
            thread::sleep(Duration::from_secs(2));
        }
    });
}

#[cfg(target_os = "windows")]
struct RecordingStatusSummary {
    short_label: String,
    tooltip: String,
    recording: bool,
    pause_resume_label: &'static str,
    stop_label: &'static str,
}

#[cfg(target_os = "windows")]
fn recording_status_summary(
    output_dir: &Path,
    language: DesktopLanguage,
) -> RecordingStatusSummary {
    let text = desktop_text(language);
    if let Some(session_id) = active_recording_session_id(output_dir) {
        let paused = read_status(output_dir, &session_id)
            .ok()
            .is_some_and(|status| matches!(status.state, crate::session::SessionState::Paused));
        if paused {
            RecordingStatusSummary {
                short_label: text.short_paused.to_string(),
                tooltip: format!("{}: {} ({session_id})", text.app_name, text.tooltip_paused),
                recording: true,
                pause_resume_label: text.resume_recording,
                stop_label: text.stop_current_recording,
            }
        } else {
            RecordingStatusSummary {
                short_label: text.short_recording.to_string(),
                tooltip: format!(
                    "{}: {} ({session_id})",
                    text.app_name, text.tooltip_recording
                ),
                recording: true,
                pause_resume_label: text.pause_recording,
                stop_label: text.stop_current_recording,
            }
        }
    } else {
        RecordingStatusSummary {
            short_label: text.short_idle.to_string(),
            tooltip: format!("{}: {}", text.app_name, text.tooltip_idle),
            recording: false,
            pause_resume_label: text.pause_resume_none,
            stop_label: text.stop_current_recording_none,
        }
    }
}

#[cfg(target_os = "windows")]
fn resolve_desktop_language(language: ViewerLanguage) -> DesktopLanguage {
    match language {
        ViewerLanguage::En => DesktopLanguage::En,
        ViewerLanguage::Zh => DesktopLanguage::Zh,
        ViewerLanguage::Auto => detect_system_language(),
    }
}

#[cfg(target_os = "windows")]
fn detect_system_language() -> DesktopLanguage {
    let language_id = unsafe { GetUserDefaultUILanguage() };
    let primary_language = language_id & 0x03ff;
    if primary_language == 0x04 {
        DesktopLanguage::Zh
    } else {
        DesktopLanguage::En
    }
}

#[cfg(target_os = "windows")]
struct DesktopText {
    app_name: &'static str,
    open_main_ui: &'static str,
    start_new_recording: &'static str,
    start_new_recording_busy: &'static str,
    pause_resume: &'static str,
    stop_current_recording: &'static str,
    stop_current_recording_none: &'static str,
    quit: &'static str,
    short_idle: &'static str,
    short_recording: &'static str,
    short_paused: &'static str,
    tooltip_idle: &'static str,
    tooltip_recording: &'static str,
    tooltip_paused: &'static str,
    pause_recording: &'static str,
    resume_recording: &'static str,
    pause_resume_none: &'static str,
}

#[cfg(target_os = "windows")]
fn desktop_text(language: DesktopLanguage) -> DesktopText {
    match language {
        DesktopLanguage::En => DesktopText {
            app_name: "Screen Timeline Recorder",
            open_main_ui: "Open Main UI",
            start_new_recording: "Start New Recording",
            start_new_recording_busy: "Start New Recording (Busy)",
            pause_resume: "Pause / Resume",
            stop_current_recording: "Stop Current Recording",
            stop_current_recording_none: "Stop Current Recording (None)",
            quit: "Quit",
            short_idle: "Idle",
            short_recording: "Recording",
            short_paused: "Paused",
            tooltip_idle: "idle",
            tooltip_recording: "recording",
            tooltip_paused: "paused",
            pause_recording: "Pause Recording",
            resume_recording: "Resume Recording",
            pause_resume_none: "Pause / Resume (No Active Recording)",
        },
        DesktopLanguage::Zh => DesktopText {
            app_name: "屏幕时间线记录器",
            open_main_ui: "打开主界面",
            start_new_recording: "开始新录制",
            start_new_recording_busy: "开始新录制（当前忙碌）",
            pause_resume: "暂停 / 继续",
            stop_current_recording: "停止当前录制",
            stop_current_recording_none: "停止当前录制（无活动录制）",
            quit: "退出",
            short_idle: "空闲",
            short_recording: "录制中",
            short_paused: "已暂停",
            tooltip_idle: "空闲",
            tooltip_recording: "录制中",
            tooltip_paused: "已暂停",
            pause_recording: "暂停录制",
            resume_recording: "继续录制",
            pause_resume_none: "暂停 / 继续（当前无活动录制）",
        },
    }
}

#[cfg(target_os = "windows")]
fn generated_session_id() -> String {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default();
    format!("session-{}", now.as_millis())
}

#[cfg(target_os = "windows")]
fn allocate_bind_addr() -> Result<String, String> {
    let listener = TcpListener::bind("127.0.0.1:0").map_err(|err| err.to_string())?;
    let addr = listener.local_addr().map_err(|err| err.to_string())?;
    drop(listener);
    Ok(format!("127.0.0.1:{}", addr.port()))
}

#[cfg(target_os = "windows")]
fn latest_session_id_or_placeholder(config: &RecorderConfig) -> String {
    get_sessions(&config.output_dir)
        .ok()
        .and_then(|sessions| sessions.into_iter().next())
        .map(|session| session.session_id)
        .unwrap_or_else(|| "desktop-empty".to_string())
}

#[cfg(target_os = "windows")]
fn resolve_icon_path() -> PathBuf {
    if let Ok(exe_path) = std::env::current_exe()
        && let Some(exe_dir) = exe_path.parent()
    {
        let packaged_icon = exe_dir.join("icons").join("icon.ico");
        if packaged_icon.is_file() {
            return packaged_icon;
        }
    }

    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("icons")
        .join("icon.ico")
}

#[cfg(not(target_os = "windows"))]
pub fn run_desktop(
    _config: RecorderConfig,
    _background: bool,
    _autorun_record: bool,
) -> Result<(), String> {
    Err("desktop mode is currently only supported on Windows".to_string())
}
