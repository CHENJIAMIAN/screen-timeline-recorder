use std::path::PathBuf;

use crate::config::{ConfigError, RecorderConfig};
use crate::recording_settings::load_recording_settings;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Command {
    RecordVideo {
        session_id: Option<String>,
    },
    View {
        session_id: String,
        bind_addr: String,
    },
    ViewLatest {
        bind_addr: String,
    },
    Desktop {
        background: bool,
        autorun_record: bool,
    },
    Pause {
        session_id: String,
    },
    Resume {
        session_id: String,
    },
    Stop {
        session_id: String,
    },
    Status {
        session_id: String,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub struct CliOptions {
    pub command: Command,
    pub config_path: Option<PathBuf>,
    pub output_dir: Option<PathBuf>,
}

impl Default for CliOptions {
    fn default() -> Self {
        Self {
            command: default_command(),
            config_path: None,
            output_dir: None,
        }
    }
}

fn default_command() -> Command {
    #[cfg(target_os = "windows")]
    {
        Command::Desktop {
            background: false,
            autorun_record: false,
        }
    }

    #[cfg(not(target_os = "windows"))]
    {
        Command::RecordVideo { session_id: None }
    }
}

impl CliOptions {
    pub fn parse_from_args<I>(args: I) -> Result<Self, String>
    where
        I: IntoIterator<Item = String>,
    {
        let mut iter = args.into_iter();
        let _program = iter.next();

        let mut options = CliOptions::default();
        let mut current = iter.next();
        while let Some(arg) = current {
            match arg.as_str() {
                "record-video" => {
                    options.command = Command::RecordVideo { session_id: None };
                }
                "view" => {
                    let session_id = iter
                        .next()
                        .ok_or_else(|| "missing session id for view command".to_string())?;
                    options.command = Command::View {
                        session_id,
                        bind_addr: "127.0.0.1:8080".to_string(),
                    };
                }
                "view-latest" => {
                    options.command = Command::ViewLatest {
                        bind_addr: "127.0.0.1:8080".to_string(),
                    };
                }
                "desktop" => {
                    options.command = Command::Desktop {
                        background: false,
                        autorun_record: false,
                    };
                }
                "pause" => {
                    let session_id = iter
                        .next()
                        .ok_or_else(|| "missing session id for pause command".to_string())?;
                    options.command = Command::Pause { session_id };
                }
                "resume" => {
                    let session_id = iter
                        .next()
                        .ok_or_else(|| "missing session id for resume command".to_string())?;
                    options.command = Command::Resume { session_id };
                }
                "stop" => {
                    let session_id = iter
                        .next()
                        .ok_or_else(|| "missing session id for stop command".to_string())?;
                    options.command = Command::Stop { session_id };
                }
                "status" => {
                    let session_id = iter
                        .next()
                        .ok_or_else(|| "missing session id for status command".to_string())?;
                    options.command = Command::Status { session_id };
                }
                "--config" => {
                    let value = iter
                        .next()
                        .ok_or_else(|| "missing value for --config".to_string())?;
                    options.config_path = Some(PathBuf::from(value));
                }
                "--output-dir" => {
                    let value = iter
                        .next()
                        .ok_or_else(|| "missing value for --output-dir".to_string())?;
                    options.output_dir = Some(PathBuf::from(value));
                }
                "--session-id" => match &mut options.command {
                    Command::RecordVideo { session_id } => {
                        *session_id = Some(
                            iter.next()
                                .ok_or_else(|| "missing value for --session-id".to_string())?,
                        );
                    }
                    _ => {
                        return Err(
                            "--session-id is only valid with the record-video command"
                                .to_string(),
                        );
                    }
                },
                "--bind" => match &mut options.command {
                    Command::View { bind_addr, .. } | Command::ViewLatest { bind_addr } => {
                        *bind_addr = iter
                            .next()
                            .ok_or_else(|| "missing value for --bind".to_string())?;
                    }
                    Command::RecordVideo { .. }
                    | Command::Desktop { .. }
                    | Command::Pause { .. }
                    | Command::Resume { .. }
                    | Command::Stop { .. }
                    | Command::Status { .. } => {
                        return Err("--bind is only valid with the view command".to_string());
                    }
                },
                "--background" => match &mut options.command {
                    Command::Desktop { background, .. } => {
                        *background = true;
                    }
                    _ => {
                        return Err(
                            "--background is only valid with the desktop command".to_string()
                        );
                    }
                },
                "--autorun-record" => match &mut options.command {
                    Command::Desktop { autorun_record, .. } => {
                        *autorun_record = true;
                    }
                    _ => {
                        return Err(
                            "--autorun-record is only valid with the desktop command".to_string()
                        );
                    }
                },
                _ => return Err(format!("unknown argument: {arg}")),
            }
            current = iter.next();
        }

        Ok(options)
    }
}

pub fn load_config(options: &CliOptions) -> Result<RecorderConfig, ConfigError> {
    let mut config = match &options.config_path {
        Some(path) => RecorderConfig::from_path(path)?,
        None => RecorderConfig::default(),
    };

    if let Some(output_dir) = &options.output_dir {
        config = config.with_output_dir(output_dir.clone());
    }

    if options.config_path.is_none() {
        let settings = load_recording_settings(&config.output_dir)
            .map_err(|err| ConfigError::Settings(err.to_string()))?;
        settings.apply_to_config(&mut config);
    }

    config.validate()?;
    Ok(config)
}
