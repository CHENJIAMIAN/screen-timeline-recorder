#![cfg_attr(all(target_os = "windows", not(debug_assertions)), windows_subsystem = "windows")]

use screen_timeline_recorder::{
    cli::{CliOptions, Command, load_config},
    desktop::run_desktop,
    dpi::initialize_process_dpi_awareness,
    retention::enforce_retention,
    session_control::{pause_session, render_status_json, resume_session, stop_session},
    video_recorder::record_video_command,
    viewer_api::get_sessions,
    viewer_server::ViewerServer,
};
use std::time::{SystemTime, UNIX_EPOCH};

fn main() {
    initialize_process_dpi_awareness();

    let options = match CliOptions::parse_from_args(std::env::args()) {
        Ok(options) => options,
        Err(error) => {
            eprintln!("{error}");
            std::process::exit(1);
        }
    };

    let config = match load_config(&options) {
        Ok(config) => config,
        Err(error) => {
            eprintln!("{error}");
            std::process::exit(1);
        }
    };

    match &options.command {
        Command::RecordVideo { session_id } => {
            let now_timestamp_ms = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis() as u64;
            if let Err(error) = enforce_retention(
                &config.output_dir,
                config.max_sessions.map(|limit| limit as usize),
                config.max_age_days,
                config.max_total_bytes,
                now_timestamp_ms,
            ) {
                eprintln!("{error}");
                std::process::exit(1);
            }
            let session_id = session_id.clone().unwrap_or_else(default_session_id);
            if let Err(error) = record_video_command(config, &session_id) {
                eprintln!("{error}");
                std::process::exit(1);
            }
        }
        Command::View {
            session_id,
            bind_addr,
        } => {
            let server = ViewerServer::new(config.output_dir.clone(), session_id.clone());
            if let Err(error) = server.serve(bind_addr) {
                eprintln!("{error}");
                std::process::exit(1);
            }
        }
        Command::ViewLatest { bind_addr } => {
            let session_id = match latest_session_id(&config.output_dir) {
                Ok(session_id) => session_id,
                Err(error) => {
                    eprintln!("{error}");
                    std::process::exit(1);
                }
            };
            let server = ViewerServer::new(config.output_dir.clone(), session_id);
            if let Err(error) = server.serve(bind_addr) {
                eprintln!("{error}");
                std::process::exit(1);
            }
        }
        Command::Desktop {
            background,
            autorun_record,
        } => {
            if let Err(error) = run_desktop(config, *background, *autorun_record) {
                eprintln!("{error}");
                std::process::exit(1);
            }
        }
        Command::Pause { session_id } => {
            if let Err(error) = pause_session(&config.output_dir, session_id) {
                eprintln!("{error}");
                std::process::exit(1);
            }
            println!("paused {session_id}");
        }
        Command::Resume { session_id } => {
            if let Err(error) = resume_session(&config.output_dir, session_id) {
                eprintln!("{error}");
                std::process::exit(1);
            }
            println!("resumed {session_id}");
        }
        Command::Stop { session_id } => {
            if let Err(error) = stop_session(&config.output_dir, session_id) {
                eprintln!("{error}");
                std::process::exit(1);
            }
            println!("stopping {session_id}");
        }
        Command::Status { session_id } => {
            match render_status_json(&config.output_dir, session_id) {
                Ok(status) => println!("{status}"),
                Err(error) => {
                    eprintln!("{error}");
                    std::process::exit(1);
                }
            }
        }
    };
}

fn default_session_id() -> String {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default();
    format!("session-{}", now.as_millis())
}

fn latest_session_id(output_dir: &std::path::Path) -> Result<String, String> {
    let sessions = get_sessions(output_dir).map_err(|error| error.to_string())?;
    sessions
        .into_iter()
        .next()
        .map(|session| session.session_id)
        .ok_or_else(|| "no sessions found under output/sessions".to_string())
}
