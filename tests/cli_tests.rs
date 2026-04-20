use std::path::PathBuf;

use screen_timeline_recorder::cli::{CliOptions, Command};

fn parse(args: &[&str]) -> Result<CliOptions, String> {
    CliOptions::parse_from_args(args.iter().map(|arg| arg.to_string()))
}

#[cfg(target_os = "windows")]
#[test]
fn defaults_to_desktop_command_on_windows_when_no_subcommand_is_given() {
    let options = parse(&["screen-timeline-recorder"]).expect("parse");

    assert_eq!(
        options.command,
        Command::Desktop {
            background: false,
            autorun_record: false,
        }
    );
}

#[test]
fn parses_view_command_with_session_id() {
    let options = parse(&["screen-timeline-recorder", "view", "2026-04-13"]).expect("parse");

    assert_eq!(
        options.command,
        Command::View {
            session_id: "2026-04-13".to_string(),
            bind_addr: "127.0.0.1:8080".to_string(),
        }
    );
}

#[test]
fn parses_view_latest_command() {
    let options = parse(&["screen-timeline-recorder", "view-latest"]).expect("parse");

    assert_eq!(
        options.command,
        Command::ViewLatest {
            bind_addr: "127.0.0.1:8080".to_string(),
        }
    );
}

#[test]
fn parses_desktop_command() {
    let options = parse(&["screen-timeline-recorder", "desktop"]).expect("parse");

    assert_eq!(
        options.command,
        Command::Desktop {
            background: false,
            autorun_record: false,
        }
    );
}

#[test]
fn parses_desktop_command_with_background_flags() {
    let options = parse(&[
        "screen-timeline-recorder",
        "desktop",
        "--background",
        "--autorun-record",
    ])
    .expect("parse");

    assert_eq!(
        options.command,
        Command::Desktop {
            background: true,
            autorun_record: true,
        }
    );
}

#[test]
fn rejects_autostart_flag_by_default() {
    let err = parse(&[
        "screen-timeline-recorder",
        "view",
        "2026-04-13",
        "--autostart",
    ])
    .unwrap_err();
    assert_eq!(err, "unknown argument: --autostart");
}

#[test]
fn parses_record_video_command_with_output_dir() {
    let options = parse(&[
        "screen-timeline-recorder",
        "record-video",
        "--output-dir",
        "D:/captures",
    ])
    .expect("parse");

    assert_eq!(options.command, Command::RecordVideo { session_id: None });
    assert_eq!(options.output_dir, Some(PathBuf::from("D:/captures")));
}

#[test]
fn parses_record_video_command_with_explicit_session_id() {
    let options = parse(&[
        "screen-timeline-recorder",
        "record-video",
        "--session-id",
        "session-video",
    ])
    .expect("parse");

    assert_eq!(
        options.command,
        Command::RecordVideo {
            session_id: Some("session-video".to_string()),
        }
    );
}

#[test]
fn parses_pause_resume_stop_and_status_commands() {
    let pause = parse(&["screen-timeline-recorder", "pause", "session-alpha"]).expect("pause");
    let resume = parse(&["screen-timeline-recorder", "resume", "session-alpha"]).expect("resume");
    let stop = parse(&["screen-timeline-recorder", "stop", "session-alpha"]).expect("stop");
    let status = parse(&["screen-timeline-recorder", "status", "session-alpha"]).expect("status");

    assert_eq!(
        pause.command,
        Command::Pause {
            session_id: "session-alpha".to_string(),
        }
    );
    assert_eq!(
        resume.command,
        Command::Resume {
            session_id: "session-alpha".to_string(),
        }
    );
    assert_eq!(
        stop.command,
        Command::Stop {
            session_id: "session-alpha".to_string(),
        }
    );
    assert_eq!(
        status.command,
        Command::Status {
            session_id: "session-alpha".to_string(),
        }
    );
}

#[test]
fn parses_view_command_with_bind_and_output_dir() {
    let options = parse(&[
        "screen-timeline-recorder",
        "view",
        "2026-04-13",
        "--bind",
        "127.0.0.1:9090",
        "--output-dir",
        "D:/captures",
    ])
    .expect("parse");

    assert_eq!(
        options.command,
        Command::View {
            session_id: "2026-04-13".to_string(),
            bind_addr: "127.0.0.1:9090".to_string(),
        }
    );
    assert_eq!(options.output_dir, Some(PathBuf::from("D:/captures")));
}

#[test]
fn parses_view_latest_command_with_bind_and_output_dir() {
    let options = parse(&[
        "screen-timeline-recorder",
        "view-latest",
        "--bind",
        "127.0.0.1:9090",
        "--output-dir",
        "D:/captures",
    ])
    .expect("parse");

    assert_eq!(
        options.command,
        Command::ViewLatest {
            bind_addr: "127.0.0.1:9090".to_string(),
        }
    );
    assert_eq!(options.output_dir, Some(PathBuf::from("D:/captures")));
}

#[test]
fn rejects_bind_without_view_command() {
    let err = parse(&["screen-timeline-recorder", "--bind", "127.0.0.1:9090"])
        .expect_err("expected bind validation error");

    assert!(err.contains("--bind is only valid with the view command"));
}

#[test]
fn rejects_view_without_session_id() {
    let err = parse(&["screen-timeline-recorder", "view"])
        .expect_err("expected missing session id error");

    assert!(err.contains("missing session id for view command"));
}

#[test]
fn rejects_pause_without_session_id() {
    let err = parse(&["screen-timeline-recorder", "pause"])
        .expect_err("expected missing session id error");

    assert!(err.contains("missing session id for pause command"));
}
