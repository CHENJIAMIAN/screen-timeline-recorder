use std::{
    fs,
    path::PathBuf,
    process::Command,
    thread,
    time::{SystemTime, UNIX_EPOCH},
};

#[cfg(windows)]
use std::os::windows::process::CommandExt;

use serde::Serialize;
use tiny_http::{Header, Method, Response, Server, StatusCode};

use crate::autostart::{AutostartSettings, apply_autostart_settings, get_autostart_status};
use crate::recording_settings::{
    RecordingSettings, apply_recording_settings, load_recording_settings,
};
use crate::session::SessionStatus;
use crate::session_control::{
    SessionControlError, delete_session, pause_session, resume_session, stop_session,
};
use crate::video_recorder::resolve_ffmpeg_path;
use crate::viewer_api::{
    get_activity, get_session, get_sessions, get_status, get_video_segments,
};

#[cfg(windows)]
const CREATE_NO_WINDOW: u32 = 0x0800_0000;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ContentType {
    Html,
    JavaScript,
    Css,
    Json,
    Mp4,
    Text,
}

impl ContentType {
    fn as_str(self) -> &'static str {
        match self {
            Self::Html => "text/html; charset=utf-8",
            Self::JavaScript => "application/javascript; charset=utf-8",
            Self::Css => "text/css; charset=utf-8",
            Self::Json => "application/json; charset=utf-8",
            Self::Mp4 => "video/mp4",
            Self::Text => "text/plain; charset=utf-8",
        }
    }
}

#[derive(Debug)]
pub struct ViewerResponse {
    pub status_code: u16,
    pub content_type: ContentType,
    pub body: Vec<u8>,
}

#[derive(Debug, Serialize)]
struct ControlResponse {
    ok: bool,
    action: String,
    session_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    status: Option<SessionStatus>,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<String>,
}

pub struct ViewerServer {
    output_dir: PathBuf,
    session_id: String,
    viewer_dir: PathBuf,
}

impl ViewerServer {
    pub fn new(output_dir: impl Into<PathBuf>, session_id: impl Into<String>) -> Self {
        let output_dir = output_dir.into();
        Self {
            output_dir: output_dir.clone(),
            session_id: session_id.into(),
            viewer_dir: resolve_viewer_dir(),
        }
    }

    pub fn handle_get(&self, path: &str) -> Result<ViewerResponse, String> {
        let (route, query) = split_path_and_query(path);
        let session_id = self.resolve_session_id(query);
        match route {
            "/" => self.serve_static("index.html", ContentType::Html),
            route if route.ends_with(".js") || route.ends_with(".css") => {
                self.serve_viewer_asset(route)
            }
            route if route.starts_with("/segments/") => {
                let asset_session_id = self.resolve_session_asset_session_id(route, query);
                self.serve_session_asset(route, &asset_session_id, ContentType::Mp4)
            }
            "/api/session" => {
                let session =
                    get_session(&self.output_dir, &session_id).map_err(|err| err.to_string())?;
                let body = serde_json::to_vec(&session).map_err(|err| err.to_string())?;
                Ok(ViewerResponse {
                    status_code: 200,
                    content_type: ContentType::Json,
                    body,
                })
            }
            "/api/sessions" => {
                let sessions = get_sessions(&self.output_dir).map_err(|err| err.to_string())?;
                let body = serde_json::to_vec(&sessions).map_err(|err| err.to_string())?;
                Ok(ViewerResponse {
                    status_code: 200,
                    content_type: ContentType::Json,
                    body,
                })
            }
            "/api/status" => {
                let status =
                    get_status(&self.output_dir, &session_id).map_err(|err| err.to_string())?;
                let body = serde_json::to_vec(&status).map_err(|err| err.to_string())?;
                Ok(ViewerResponse {
                    status_code: 200,
                    content_type: ContentType::Json,
                    body,
                })
            }
            "/api/autostart" => {
                let status =
                    get_autostart_status(&self.output_dir).map_err(|err| err.to_string())?;
                let body = serde_json::to_vec(&status).map_err(|err| err.to_string())?;
                Ok(ViewerResponse {
                    status_code: 200,
                    content_type: ContentType::Json,
                    body,
                })
            }
            "/api/autostart/save" => self.handle_autostart_save(query),
            "/api/recording-settings" => self.handle_recording_settings(),
            "/api/recording-settings/save" => self.handle_recording_settings_save(query),
            "/api/control" => self.handle_control(query, &session_id),
            "/api/activity" => {
                let activity =
                    get_activity(&self.output_dir, &session_id).map_err(|err| err.to_string())?;
                let body = serde_json::to_vec(&activity).map_err(|err| err.to_string())?;
                Ok(ViewerResponse {
                    status_code: 200,
                    content_type: ContentType::Json,
                    body,
                })
            }
            "/api/segments" => {
                let segments = get_video_segments(&self.output_dir, &session_id)
                    .map_err(|err| err.to_string())?;
                let body = serde_json::to_vec(&segments).map_err(|err| err.to_string())?;
                Ok(ViewerResponse {
                    status_code: 200,
                    content_type: ContentType::Json,
                    body,
                })
            }
            _ => Ok(ViewerResponse {
                status_code: 404,
                content_type: ContentType::Text,
                body: b"not found".to_vec(),
            }),
        }
    }

    fn resolve_session_id(&self, query: &str) -> String {
        if let Some(requested) = session_id_from_query(query)
            && self.session_exists(requested)
        {
            return requested.to_string();
        }

        if self.session_exists(&self.session_id) {
            return self.session_id.clone();
        }

        get_sessions(&self.output_dir)
            .ok()
            .and_then(|sessions| sessions.into_iter().next())
            .map(|session| session.session_id)
            .unwrap_or_else(|| self.session_id.clone())
    }

    fn session_exists(&self, session_id: &str) -> bool {
        self.output_dir
            .join("sessions")
            .join(session_id)
            .join("manifest.json")
            .is_file()
    }

    fn resolve_session_asset_session_id(&self, route: &str, query: &str) -> String {
        let relative = route.trim_start_matches('/');
        if let Some(requested) = session_id_from_query(query) {
            let requested_asset = self
                .output_dir
                .join("sessions")
                .join(requested)
                .join(relative);
            if requested_asset.is_file() {
                return requested.to_string();
            }
        }

        self.resolve_session_id(query)
    }

    pub fn serve(&self, bind_addr: &str) -> Result<(), String> {
        let server = Server::http(bind_addr).map_err(|err| err.to_string())?;
        for request in server.incoming_requests() {
            if request.method() != &Method::Get {
                let response = Response::from_string("method not allowed")
                    .with_status_code(StatusCode(405))
                    .with_header(content_type_header(ContentType::Text)?);
                let _ = request.respond(response);
                continue;
            }

            let response = match self.handle_get(request.url()) {
                Ok(viewer_response) => Response::from_data(viewer_response.body)
                    .with_status_code(StatusCode(viewer_response.status_code))
                    .with_header(content_type_header(viewer_response.content_type)?),
                Err(err) => Response::from_string(err)
                    .with_status_code(StatusCode(500))
                    .with_header(content_type_header(ContentType::Text)?),
            };
            let _ = request.respond(response);
        }
        Ok(())
    }

    fn serve_static(
        &self,
        relative_path: &str,
        content_type: ContentType,
    ) -> Result<ViewerResponse, String> {
        let path = self.viewer_dir.join(relative_path);
        let body = fs::read(path).map_err(|err| err.to_string())?;
        Ok(ViewerResponse {
            status_code: 200,
            content_type,
            body,
        })
    }

    fn serve_session_asset(
        &self,
        route: &str,
        session_id: &str,
        content_type: ContentType,
    ) -> Result<ViewerResponse, String> {
        let relative = route.trim_start_matches('/');
        let path = self
            .output_dir
            .join("sessions")
            .join(session_id)
            .join(relative);
        let body = fs::read(path).map_err(|err| err.to_string())?;
        Ok(ViewerResponse {
            status_code: 200,
            content_type,
            body,
        })
    }

    fn serve_viewer_asset(&self, route: &str) -> Result<ViewerResponse, String> {
        let relative = route.trim_start_matches('/');
        if relative.contains("..") {
            return Err("invalid asset path".to_string());
        }

        let content_type = if relative.ends_with(".css") {
            ContentType::Css
        } else {
            ContentType::JavaScript
        };

        self.serve_static(relative, content_type)
    }

    fn handle_control(&self, query: &str, session_id: &str) -> Result<ViewerResponse, String> {
        let action = control_action_from_query(query).unwrap_or_default();
        let response = match action {
            "pause" => match pause_session(&self.output_dir, session_id) {
                Ok(()) => ok_control_response(
                    "pause",
                    session_id,
                    get_status(&self.output_dir, session_id).ok(),
                ),
                Err(err) => error_control_response("pause", session_id, err),
            },
            "resume" => match resume_session(&self.output_dir, session_id) {
                Ok(()) => ok_control_response(
                    "resume",
                    session_id,
                    get_status(&self.output_dir, session_id).ok(),
                ),
                Err(err) => error_control_response("resume", session_id, err),
            },
            "stop" => match stop_session(&self.output_dir, session_id) {
                Ok(()) => ok_control_response(
                    "stop",
                    session_id,
                    wait_for_session_stop(&self.output_dir, session_id).ok(),
                ),
                Err(err) => error_control_response("stop", session_id, err),
            },
            "delete" => match delete_session(&self.output_dir, session_id) {
                Ok(()) => ok_control_response("delete", session_id, None),
                Err(err) => error_control_response("delete", session_id, err),
            },
            "start" => match self.start_recording_session() {
                Ok(started_session_id) => ok_control_response("start", &started_session_id, None),
                Err(error) => ViewerResponse {
                    status_code: 409,
                    content_type: ContentType::Json,
                    body: serde_json::to_vec(&ControlResponse {
                        ok: false,
                        action: "start".to_string(),
                        session_id: session_id.to_string(),
                        status: None,
                        error: Some(error),
                    })
                    .expect("serialize start control error"),
                },
            },
            "status" => match get_status(&self.output_dir, session_id) {
                Ok(status) => ok_control_response("status", session_id, Some(status)),
                Err(err) => ViewerResponse {
                    status_code: 500,
                    content_type: ContentType::Json,
                    body: serde_json::to_vec(&ControlResponse {
                        ok: false,
                        action: "status".to_string(),
                        session_id: session_id.to_string(),
                        status: None,
                        error: Some(err.to_string()),
                    })
                    .expect("serialize status control error"),
                },
            },
            _ => ViewerResponse {
                status_code: 400,
                content_type: ContentType::Json,
                body: serde_json::to_vec(&ControlResponse {
                    ok: false,
                    action: action.to_string(),
                    session_id: session_id.to_string(),
                    status: None,
                    error: Some("missing or invalid control action".to_string()),
                })
                .map_err(|err| err.to_string())?,
            },
        };
        Ok(response)
    }

    fn handle_autostart_save(&self, query: &str) -> Result<ViewerResponse, String> {
        let settings = autostart_settings_from_query(query, &self.output_dir)?;
        let status =
            apply_autostart_settings(&self.output_dir, &settings).map_err(|err| err.to_string())?;
        let body = serde_json::to_vec(&status).map_err(|err| err.to_string())?;
        Ok(ViewerResponse {
            status_code: 200,
            content_type: ContentType::Json,
            body,
        })
    }

    fn handle_recording_settings(&self) -> Result<ViewerResponse, String> {
        let settings = load_recording_settings(&self.output_dir).map_err(|err| err.to_string())?;
        let body = serde_json::to_vec(&settings).map_err(|err| err.to_string())?;
        Ok(ViewerResponse {
            status_code: 200,
            content_type: ContentType::Json,
            body,
        })
    }

    fn handle_recording_settings_save(&self, query: &str) -> Result<ViewerResponse, String> {
        let settings = recording_settings_from_query(query, &self.output_dir)?;
        let saved =
            apply_recording_settings(&self.output_dir, &settings).map_err(|err| err.to_string())?;
        let body = serde_json::to_vec(&saved).map_err(|err| err.to_string())?;
        Ok(ViewerResponse {
            status_code: 200,
            content_type: ContentType::Json,
            body,
        })
    }

    fn start_recording_session(&self) -> Result<String, String> {
        let sessions = get_sessions(&self.output_dir).map_err(|err| err.to_string())?;
        if let Some(active) = sessions.into_iter().find(|session| {
            session
                .status
                .as_ref()
                .is_some_and(|status| status.recording)
        }) {
            return Err(format!(
                "recording is already active in {}",
                active.session_id
            ));
        }

        let exe_dir = std::env::current_exe()
            .ok()
            .and_then(|path| path.parent().map(std::path::Path::to_path_buf));
        resolve_ffmpeg_path(exe_dir.as_deref(), &[]).ok_or_else(|| {
            "ffmpeg sidecar not found; expected ffmpeg\\ffmpeg.exe next to the app or SCREEN_TIMELINE_FFMPEG".to_string()
        })?;

        let session_id = format!(
            "session-{}",
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis()
        );
        let exe_path = std::env::current_exe().map_err(|err| err.to_string())?;
        let mut command = Command::new(exe_path);
        #[cfg(windows)]
        command.creation_flags(CREATE_NO_WINDOW);
        command
            .arg(recording_start_subcommand())
            .arg("--session-id")
            .arg(&session_id)
            .arg("--output-dir")
            .arg(&self.output_dir)
            .spawn()
            .map_err(|err| format!("failed to start a new recording: {err}"))?;
        Ok(session_id)
    }
}

pub(crate) fn recording_start_subcommand() -> &'static str {
    "record-video"
}

fn resolve_viewer_dir() -> PathBuf {
    if let Ok(exe_path) = std::env::current_exe()
        && let Some(exe_dir) = exe_path.parent()
    {
        let packaged_viewer_dir = exe_dir.join("viewer");
        if packaged_viewer_dir.join("index.html").is_file() {
            return packaged_viewer_dir;
        }
    }

    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("viewer")
}

fn split_path_and_query(path: &str) -> (&str, &str) {
    match path.split_once('?') {
        Some((route, query)) => (route, query),
        None => (path, ""),
    }
}

fn session_id_from_query(query: &str) -> Option<&str> {
    for pair in query.split('&') {
        if let Some((key, value)) = pair.split_once('=') {
            if key == "session_id" && !value.is_empty() {
                return Some(value);
            }
        }
    }
    None
}

fn control_action_from_query(query: &str) -> Option<&str> {
    for pair in query.split('&') {
        if let Some((key, value)) = pair.split_once('=')
            && key == "action"
            && !value.is_empty()
        {
            return Some(value);
        }
    }
    None
}

fn autostart_settings_from_query(
    query: &str,
    output_dir: &std::path::Path,
) -> Result<AutostartSettings, String> {
    let mut settings = get_autostart_status(output_dir)
        .map_err(|err| err.to_string())?
        .settings;

    for pair in query.split('&') {
        if let Some((key, value)) = pair.split_once('=') {
            let decoded = decode_query_value(value);
            match key {
                "enabled" => {
                    settings.enabled = parse_bool_flag(&decoded)?;
                }
                "start_on_login" => {
                    settings.start_on_login = parse_bool_flag(&decoded)?;
                }
                "delay_seconds" => {
                    settings.delay_seconds = decoded
                        .parse::<u32>()
                        .map_err(|_| format!("invalid delay_seconds: {decoded}"))?;
                }
                "output_dir" => {
                    if !decoded.is_empty() {
                        settings.output_dir = PathBuf::from(decoded);
                    }
                }
                _ => {}
            }
        }
    }

    Ok(settings)
}

fn recording_settings_from_query(
    query: &str,
    output_dir: &std::path::Path,
) -> Result<RecordingSettings, String> {
    let mut settings = load_recording_settings(output_dir).map_err(|err| err.to_string())?;

    for pair in query.split('&') {
        if let Some((key, value)) = pair.split_once('=') {
            let decoded = decode_query_value(value);
            match key {
                "sampling_interval_ms" => {
                    settings.sampling_interval_ms = decoded
                        .parse::<u64>()
                        .map_err(|_| format!("invalid sampling_interval_ms: {decoded}"))?;
                }
                "working_scale" => {
                    settings.working_scale = decoded
                        .parse::<f32>()
                        .map_err(|_| format!("invalid working_scale: {decoded}"))?;
                }
                "burn_in_enabled" => {
                    settings.burn_in_enabled = parse_bool_flag(&decoded)?;
                }
                _ => {}
            }
        }
    }

    Ok(settings)
}

fn parse_bool_flag(value: &str) -> Result<bool, String> {
    match value {
        "1" | "true" | "yes" | "on" => Ok(true),
        "0" | "false" | "no" | "off" => Ok(false),
        _ => Err(format!("invalid boolean flag: {value}")),
    }
}

fn decode_query_value(value: &str) -> String {
    let replaced = value.replace('+', " ");
    let bytes = replaced.as_bytes();
    let mut decoded = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%'
            && i + 2 < bytes.len()
            && let (Some(high), Some(low)) = (hex_value(bytes[i + 1]), hex_value(bytes[i + 2]))
        {
            decoded.push((high << 4) | low);
            i += 3;
            continue;
        }
        decoded.push(bytes[i]);
        i += 1;
    }
    String::from_utf8_lossy(&decoded).into_owned()
}

fn hex_value(byte: u8) -> Option<u8> {
    match byte {
        b'0'..=b'9' => Some(byte - b'0'),
        b'a'..=b'f' => Some(byte - b'a' + 10),
        b'A'..=b'F' => Some(byte - b'A' + 10),
        _ => None,
    }
}

fn content_type_header(content_type: ContentType) -> Result<Header, String> {
    Header::from_bytes("Content-Type", content_type.as_str())
        .map_err(|_| "invalid header".to_string())
}

fn wait_for_session_stop(
    output_dir: &std::path::Path,
    session_id: &str,
) -> Result<SessionStatus, String> {
    const MAX_ATTEMPTS: usize = 80;
    const POLL_MS: u64 = 100;

    for _ in 0..MAX_ATTEMPTS {
        let session = get_session(output_dir, session_id).map_err(|err| err.to_string())?;
        let status = get_status(output_dir, session_id).map_err(|err| err.to_string())?;
        if session.finished_at.is_some() && !status.recording {
            return Ok(status);
        }
        thread::sleep(std::time::Duration::from_millis(POLL_MS));
    }

    get_status(output_dir, session_id).map_err(|err| err.to_string())
}

fn ok_control_response(
    action: &str,
    session_id: &str,
    status: Option<SessionStatus>,
) -> ViewerResponse {
    ViewerResponse {
        status_code: 200,
        content_type: ContentType::Json,
        body: serde_json::to_vec(&ControlResponse {
            ok: true,
            action: action.to_string(),
            session_id: session_id.to_string(),
            status,
            error: None,
        })
        .expect("serialize control response"),
    }
}

fn error_control_response(
    action: &str,
    session_id: &str,
    err: SessionControlError,
) -> ViewerResponse {
    let status_code = match err {
        SessionControlError::MissingSession(_) => 404,
        _ => 500,
    };
    ViewerResponse {
        status_code,
        content_type: ContentType::Json,
        body: serde_json::to_vec(&ControlResponse {
            ok: false,
            action: action.to_string(),
            session_id: session_id.to_string(),
            status: None,
            error: Some(err.to_string()),
        })
        .expect("serialize control error response"),
    }
}

#[cfg(test)]
mod tests {
    use super::{
        recording_settings_from_query, recording_start_subcommand, resolve_viewer_dir,
        wait_for_session_stop,
    };

    #[cfg(windows)]
    use super::CREATE_NO_WINDOW;
    use crate::{
        recording_settings::RecordingSettings,
        session::{SessionState, SessionStatus},
    };
    use serde_json::json;
    use std::{fs, thread, time::Duration};

    #[test]
    fn resolve_viewer_dir_falls_back_to_manifest_viewer() {
        let viewer_dir = resolve_viewer_dir();

        assert!(viewer_dir.join("index.html").is_file());
        assert_eq!(
            viewer_dir.file_name().and_then(|name| name.to_str()),
            Some("viewer")
        );
    }

    #[test]
    fn control_start_uses_video_recording_subcommand() {
        assert_eq!(recording_start_subcommand(), "record-video");
    }

    #[test]
    fn recording_settings_query_only_updates_supported_video_fields() {
        let temp_dir = tempfile::tempdir().expect("tempdir");
        let settings = recording_settings_from_query(
            "sampling_interval_ms=750&working_scale=0.6&burn_in_enabled=0&block_width=8&block_height=8&keyframe_interval_ms=12000&sensitivity_mode=detailed",
            temp_dir.path(),
        )
        .expect("parse recording settings");
        let defaults = RecordingSettings::defaults();

        assert_eq!(settings.sampling_interval_ms, 750);
        assert_eq!(settings.working_scale, 0.6);
        assert!(!settings.burn_in_enabled);
        assert_eq!(settings.block_width, defaults.block_width);
        assert_eq!(settings.block_height, defaults.block_height);
        assert_eq!(settings.keyframe_interval_ms, defaults.keyframe_interval_ms);
        assert_eq!(settings.sensitivity_mode, defaults.sensitivity_mode);
    }

    #[cfg(windows)]
    #[test]
    fn control_start_uses_no_window_creation_flag() {
        assert_eq!(CREATE_NO_WINDOW, 0x0800_0000);
    }

    #[test]
    fn wait_for_session_stop_blocks_until_manifest_is_finalized() {
        let temp_dir = tempfile::tempdir().expect("tempdir");
        let session_root = temp_dir.path().join("sessions").join("session-stop");
        fs::create_dir_all(&session_root).expect("session dir");
        fs::write(
            session_root.join("manifest.json"),
            serde_json::to_vec_pretty(&json!({
                "session_id": "session-stop",
                "started_at": 1000,
                "finished_at": null,
                "display_width": 1920,
                "display_height": 1080,
                "video_width": 960,
                "video_height": 540,
                "recording_format": "video-segments",
                "segment_duration_ms": 30000,
                "video_codec": "h264",
                "recorder_version": "0.1.0",
                "viewer_default_zoom": 1.0,
                "viewer_overlay_enabled_by_default": false,
                "burn_in_enabled": true,
                "viewer_language": "auto"
            }))
            .expect("manifest json"),
        )
        .expect("write manifest");
        fs::write(
            session_root.join("status.json"),
            serde_json::to_vec_pretty(&SessionStatus {
                session_id: "session-stop".to_string(),
                state: SessionState::Running,
                recording: true,
                stats: crate::recorder::RecordingStats {
                    started_at: 1000,
                    finished_at: 1000,
                    ..Default::default()
                },
            })
            .expect("status json"),
        )
        .expect("write status");
        fs::write(session_root.join("stop.signal"), b"stop").expect("write stop signal");

        let manifest_path = session_root.join("manifest.json");
        let status_path = session_root.join("status.json");
        thread::spawn(move || {
            thread::sleep(Duration::from_millis(200));
            fs::write(
                &manifest_path,
                serde_json::to_vec_pretty(&json!({
                    "session_id": "session-stop",
                    "started_at": 1000,
                    "finished_at": 1300,
                    "display_width": 1920,
                    "display_height": 1080,
                    "video_width": 960,
                    "video_height": 540,
                    "recording_format": "video-segments",
                    "segment_duration_ms": 30000,
                    "video_codec": "h264",
                    "recorder_version": "0.1.0",
                    "viewer_default_zoom": 1.0,
                    "viewer_overlay_enabled_by_default": false,
                    "burn_in_enabled": true,
                    "viewer_language": "auto"
                }))
                .expect("manifest json"),
            )
            .expect("update manifest");
            fs::write(
                &status_path,
                serde_json::to_vec_pretty(&SessionStatus {
                    session_id: "session-stop".to_string(),
                    state: SessionState::Stopped,
                    recording: false,
                    stats: crate::recorder::RecordingStats {
                        started_at: 1000,
                        finished_at: 1300,
                        ..Default::default()
                    },
                })
                .expect("status json"),
            )
            .expect("update status");
        });

        let status = wait_for_session_stop(temp_dir.path(), "session-stop").expect("wait");
        assert_eq!(status.state, SessionState::Stopped);
        assert!(!status.recording);
        assert_eq!(status.stats.finished_at, 1300);
    }
}
