use std::{
    fs,
    path::PathBuf,
    process::Command,
    time::{SystemTime, UNIX_EPOCH},
};

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
use crate::viewer_api::{
    get_activity, get_frame_png, get_patches, get_session, get_sessions, get_status,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ContentType {
    Html,
    JavaScript,
    Css,
    Json,
    Png,
    Text,
}

impl ContentType {
    fn as_str(self) -> &'static str {
        match self {
            Self::Html => "text/html; charset=utf-8",
            Self::JavaScript => "application/javascript; charset=utf-8",
            Self::Css => "text/css; charset=utf-8",
            Self::Json => "application/json; charset=utf-8",
            Self::Png => "image/png",
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
        let session_id = session_id_from_query(query).unwrap_or(&self.session_id);
        match route {
            "/" => self.serve_static("index.html", ContentType::Html),
            "/app.js" => self.serve_static("app.js", ContentType::JavaScript),
            "/control_logic.js" => self.serve_static("control_logic.js", ContentType::JavaScript),
            "/session_list_logic.js" => {
                self.serve_static("session_list_logic.js", ContentType::JavaScript)
            }
            "/styles.css" => self.serve_static("styles.css", ContentType::Css),
            "/api/session" => {
                let session =
                    get_session(&self.output_dir, session_id).map_err(|err| err.to_string())?;
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
                    get_status(&self.output_dir, session_id).map_err(|err| err.to_string())?;
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
            "/api/control" => self.handle_control(query, session_id),
            "/api/activity" => {
                let activity =
                    get_activity(&self.output_dir, session_id).map_err(|err| err.to_string())?;
                let body = serde_json::to_vec(&activity).map_err(|err| err.to_string())?;
                Ok(ViewerResponse {
                    status_code: 200,
                    content_type: ContentType::Json,
                    body,
                })
            }
            "/api/frame" => {
                let timestamp_ms = parse_timestamp(query)?;
                let body = get_frame_png(&self.output_dir, session_id, timestamp_ms)
                    .map_err(|err| err.to_string())?;
                Ok(ViewerResponse {
                    status_code: 200,
                    content_type: ContentType::Png,
                    body,
                })
            }
            "/api/patches" => {
                let timestamp_ms = parse_timestamp(query)?;
                let patches = get_patches(&self.output_dir, session_id, timestamp_ms)
                    .map_err(|err| err.to_string())?;
                let body = serde_json::to_vec(&patches).map_err(|err| err.to_string())?;
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
                    get_status(&self.output_dir, session_id).ok(),
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

        let session_id = format!(
            "session-{}",
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis()
        );
        let exe_path = std::env::current_exe().map_err(|err| err.to_string())?;
        Command::new(exe_path)
            .arg("record")
            .arg("--session-id")
            .arg(&session_id)
            .arg("--output-dir")
            .arg(&self.output_dir)
            .spawn()
            .map_err(|err| format!("failed to start a new recording: {err}"))?;
        Ok(session_id)
    }
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

fn parse_timestamp(query: &str) -> Result<u64, String> {
    for pair in query.split('&') {
        if let Some((key, value)) = pair.split_once('=') {
            if key == "ts" {
                return value
                    .parse::<u64>()
                    .map_err(|_| format!("invalid timestamp: {value}"));
            }
        }
    }
    Err("missing ts query parameter".to_string())
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
                "block_width" => {
                    settings.block_width = decoded
                        .parse::<u32>()
                        .map_err(|_| format!("invalid block_width: {decoded}"))?;
                }
                "block_height" => {
                    settings.block_height = decoded
                        .parse::<u32>()
                        .map_err(|_| format!("invalid block_height: {decoded}"))?;
                }
                "keyframe_interval_ms" => {
                    settings.keyframe_interval_ms = decoded
                        .parse::<u64>()
                        .map_err(|_| format!("invalid keyframe_interval_ms: {decoded}"))?;
                }
                "working_scale" => {
                    settings.working_scale = decoded
                        .parse::<f32>()
                        .map_err(|_| format!("invalid working_scale: {decoded}"))?;
                }
                "burn_in_enabled" => {
                    settings.burn_in_enabled = parse_bool_flag(&decoded)?;
                }
                "sensitivity_mode" => {
                    settings.sensitivity_mode = match decoded.as_str() {
                        "conservative" => crate::config::SensitivityMode::Conservative,
                        "balanced" => crate::config::SensitivityMode::Balanced,
                        "detailed" => crate::config::SensitivityMode::Detailed,
                        _ => return Err(format!("invalid sensitivity_mode: {decoded}")),
                    };
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
    use super::resolve_viewer_dir;

    #[test]
    fn resolve_viewer_dir_falls_back_to_manifest_viewer() {
        let viewer_dir = resolve_viewer_dir();

        assert!(viewer_dir.join("index.html").is_file());
        assert_eq!(
            viewer_dir.file_name().and_then(|name| name.to_str()),
            Some("viewer")
        );
    }
}
