use std::{
    fs,
    path::{Path, PathBuf},
    process::{Child, ChildStdin, Command, Stdio},
    thread,
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};
#[cfg(windows)]
use std::os::windows::process::CommandExt;
#[cfg(windows)]
use windows::Win32::UI::WindowsAndMessaging::{GetSystemMetrics, SM_CXSCREEN, SM_CYSCREEN};

use crate::{
    config::RecorderConfig,
    recording_stats::RecordingStats,
    session::{RecordingFormat, SessionLayout, SessionState, SessionStatus},
    video_session::{VideoSegmentEntry, VideoSessionManifest, append_video_segment_index},
};

const DEFAULT_SEGMENT_DURATION_SECS: u64 = 30;
const DEFAULT_CRF: u8 = 34;
const BURN_IN_FONT_SIZE: u32 = 22;
const DEFAULT_PRIMARY_DISPLAY_WIDTH: u32 = 1920;
const DEFAULT_PRIMARY_DISPLAY_HEIGHT: u32 = 1080;
#[cfg(windows)]
const CREATE_NO_WINDOW: u32 = 0x0800_0000;

pub fn record_video_command(config: RecorderConfig, session_id: &str) -> Result<(), String> {
    let exe_dir = std::env::current_exe()
        .ok()
        .and_then(|path| path.parent().map(Path::to_path_buf));
    let ffmpeg_path = resolve_ffmpeg_path(exe_dir.as_deref(), &[])
        .ok_or_else(|| "ffmpeg sidecar not found; expected ffmpeg\\ffmpeg.exe next to the app or SCREEN_TIMELINE_FFMPEG".to_string())?;

    let layout = SessionLayout::new(&config.output_dir, session_id);
    layout.create_video_dirs().map_err(|err| err.to_string())?;

    let started_at = current_timestamp_ms();
    let (display_width, display_height) = primary_display_size();
    let video_width = ((display_width as f32) * config.working_scale)
        .round()
        .clamp(1.0, display_width as f32) as u32;
    let video_height = ((display_height as f32) * config.working_scale)
        .round()
        .clamp(1.0, display_height as f32) as u32;

    let manifest = VideoSessionManifest {
        session_id: session_id.to_string(),
        started_at,
        finished_at: None,
        display_width,
        display_height,
        video_width,
        video_height,
        recording_format: RecordingFormat::VideoSegments,
        segment_duration_ms: DEFAULT_SEGMENT_DURATION_SECS * 1_000,
        video_codec: "h264".to_string(),
        recorder_version: env!("CARGO_PKG_VERSION").to_string(),
        viewer_default_zoom: config.viewer_default_zoom,
        viewer_overlay_enabled_by_default: false,
        burn_in_enabled: config.burn_in_enabled,
        viewer_language: config.viewer_language,
    };
    fs::write(
        layout.manifest_path(),
        serde_json::to_vec_pretty(&manifest).map_err(|err| err.to_string())?,
    )
    .map_err(|err| err.to_string())?;
    write_status(
        &layout,
        SessionState::Running,
        RecordingStats {
            started_at,
            finished_at: started_at,
            ..RecordingStats::default()
        },
    )?;

    let args = build_ffmpeg_segment_args(
        &config,
        &layout,
        display_width,
        display_height,
        video_width,
        video_height,
        DEFAULT_SEGMENT_DURATION_SECS,
        &ffmpeg_path,
    );
    let mut command = Command::new(&ffmpeg_path);
    #[cfg(windows)]
    command.creation_flags(CREATE_NO_WINDOW);
    let mut child = command
        .args(&args)
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|err| err.to_string())?;

    loop {
        if layout.stop_signal_path().exists() {
            request_ffmpeg_stop(&mut child).map_err(|err| err.to_string())?;
            break;
        }

        if let Some(status) = child.try_wait().map_err(|err| err.to_string())? {
            if !status.success() {
                return Err(format!("ffmpeg exited with status {status}"));
            }
            break;
        }

        thread::sleep(Duration::from_millis(250));
    }

    let finished_at = current_timestamp_ms();
    let segment_entries =
        build_video_segment_index(&layout, started_at, finished_at, manifest.segment_duration_ms)
            .map_err(|err| err.to_string())?;
    write_video_segment_index(&layout, &segment_entries)?;
    write_status(
        &layout,
        SessionState::Stopped,
        RecordingStats {
            started_at,
            finished_at,
            ..RecordingStats::default()
        },
    )?;
    finalize_manifest(&layout, finished_at)?;
    Ok(())
}

pub fn resolve_ffmpeg_path(
    exe_dir: Option<&Path>,
    explicit_candidates: &[PathBuf],
) -> Option<PathBuf> {
    for candidate in explicit_candidates {
        if candidate.is_file() {
            return Some(candidate.clone());
        }
    }

    if let Ok(env_path) = std::env::var("SCREEN_TIMELINE_FFMPEG") {
        let env_path = PathBuf::from(env_path);
        if env_path.is_file() {
            return Some(env_path);
        }
    }

    let Some(exe_dir) = exe_dir else {
        return None;
    };

    let bundled_candidates = [
        exe_dir.join("ffmpeg").join("ffmpeg.exe"),
        exe_dir.join("ffmpeg.exe"),
        exe_dir.join("tools").join("ffmpeg.exe"),
    ];
    bundled_candidates.into_iter().find(|path| path.is_file())
}

pub fn build_ffmpeg_segment_args(
    config: &RecorderConfig,
    layout: &SessionLayout,
    display_width: u32,
    display_height: u32,
    video_width: u32,
    video_height: u32,
    segment_duration_secs: u64,
    _ffmpeg_path: &Path,
) -> Vec<String> {
    let framerate = ((1_000f32 / config.sampling_interval_ms.max(1) as f32).floor() as u64).max(1);
    let segment_duration_secs = segment_duration_secs.max(DEFAULT_SEGMENT_DURATION_SECS);
    let video_filter = build_video_filter(config, display_width, display_height, video_width, video_height);
    let segment_pattern = layout.segments_dir().join("%06d.mp4");

    vec![
        "-hide_banner".to_string(),
        "-loglevel".to_string(),
        "error".to_string(),
        "-fflags".to_string(),
        "nobuffer".to_string(),
        "-f".to_string(),
        "gdigrab".to_string(),
        "-framerate".to_string(),
        framerate.to_string(),
        "-video_size".to_string(),
        format!("{display_width}x{display_height}"),
        "-rtbufsize".to_string(),
        "32M".to_string(),
        "-draw_mouse".to_string(),
        "1".to_string(),
        "-i".to_string(),
        "desktop".to_string(),
        "-vf".to_string(),
        video_filter,
        "-c:v".to_string(),
        "libx264".to_string(),
        "-tune".to_string(),
        "zerolatency".to_string(),
        "-preset".to_string(),
        "veryfast".to_string(),
        "-threads".to_string(),
        "2".to_string(),
        "-x264-params".to_string(),
        "rc-lookahead=0:sync-lookahead=0".to_string(),
        "-crf".to_string(),
        DEFAULT_CRF.to_string(),
        "-pix_fmt".to_string(),
        "yuv420p".to_string(),
        "-g".to_string(),
        (framerate * segment_duration_secs).to_string(),
        "-sc_threshold".to_string(),
        "0".to_string(),
        "-f".to_string(),
        "segment".to_string(),
        "-segment_time".to_string(),
        segment_duration_secs.to_string(),
        "-reset_timestamps".to_string(),
        "1".to_string(),
        segment_pattern.to_string_lossy().to_string(),
    ]
}

fn primary_display_size() -> (u32, u32) {
    #[cfg(windows)]
    {
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
    #[cfg(not(windows))]
    {
        (DEFAULT_PRIMARY_DISPLAY_WIDTH, DEFAULT_PRIMARY_DISPLAY_HEIGHT)
    }
}

fn build_video_filter(
    config: &RecorderConfig,
    display_width: u32,
    display_height: u32,
    video_width: u32,
    video_height: u32,
) -> String {
    let mut filters = Vec::new();
    if video_width != display_width || video_height != display_height {
        filters.push(format!("scale={video_width}:{video_height}"));
    }

    if config.burn_in_enabled {
        let fontfile = resolve_burn_in_font()
            .map(|path| format!("fontfile='{}':", path.replace('\\', "/").replace(':', "\\:")))
            .unwrap_or_default();
        filters.push(format!(
            "drawtext={fontfile}text='%{{localtime\\:%Y-%m-%d %H-%M-%S}}':x=24:y=24:fontsize={BURN_IN_FONT_SIZE}:fontcolor=white:borderw=2:bordercolor=black@0.85:box=1:boxcolor=black@0.35:boxborderw=10"
        ));
    }

    if filters.is_empty() {
        "null".to_string()
    } else {
        filters.join(",")
    }
}

fn resolve_burn_in_font() -> Option<String> {
    let candidates = [
        "C:\\Windows\\Fonts\\consola.ttf",
        "C:\\Windows\\Fonts\\arial.ttf",
        "C:\\Windows\\Fonts\\segoeui.ttf",
    ];
    candidates
        .into_iter()
        .find(|path| Path::new(path).is_file())
        .map(str::to_string)
}

pub fn build_video_segment_index(
    layout: &SessionLayout,
    started_at: u64,
    finished_at: u64,
    segment_duration_ms: u64,
) -> Result<Vec<VideoSegmentEntry>, std::io::Error> {
    let mut files: Vec<_> = fs::read_dir(layout.segments_dir())?
        .filter_map(|entry| entry.ok())
        .filter(|entry| entry.file_type().map(|kind| kind.is_file()).unwrap_or(false))
        .collect();
    files.sort_by_key(|entry| entry.file_name());

    let mut entries = Vec::new();
    for (index, entry) in files.into_iter().enumerate() {
        let metadata = entry.metadata()?;
        let sequence = index as u64;
        let segment_started_at = started_at + sequence * segment_duration_ms;
        let nominal_finished_at = segment_started_at + segment_duration_ms;
        entries.push(VideoSegmentEntry {
            sequence,
            started_at: segment_started_at,
            finished_at: Some(nominal_finished_at.min(finished_at)),
            relative_path: format!("segments/{}", entry.file_name().to_string_lossy()),
            bytes: metadata.len(),
        });
    }
    Ok(entries)
}

fn current_timestamp_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

fn write_status(
    layout: &SessionLayout,
    state: SessionState,
    stats: RecordingStats,
) -> Result<(), String> {
    fs::write(
        layout.status_path(),
        serde_json::to_vec_pretty(&SessionStatus {
            session_id: layout
                .root()
                .file_name()
                .and_then(|value| value.to_str())
                .unwrap_or_default()
                .to_string(),
            state,
            recording: state != SessionState::Stopped,
            stats,
        })
        .map_err(|err| err.to_string())?,
    )
    .map_err(|err| err.to_string())
}

fn finalize_manifest(layout: &SessionLayout, finished_at: u64) -> Result<(), String> {
    let mut manifest: VideoSessionManifest = serde_json::from_slice(
        &fs::read(layout.manifest_path()).map_err(|err| err.to_string())?,
    )
    .map_err(|err| err.to_string())?;
    manifest.finished_at = Some(finished_at);
    fs::write(
        layout.manifest_path(),
        serde_json::to_vec_pretty(&manifest).map_err(|err| err.to_string())?,
    )
    .map_err(|err| err.to_string())
}

fn write_video_segment_index(
    layout: &SessionLayout,
    entries: &[VideoSegmentEntry],
) -> Result<(), String> {
    let index_path = layout.index_dir().join("segments.jsonl");
    if index_path.exists() {
        fs::remove_file(&index_path).map_err(|err| err.to_string())?;
    }
    for entry in entries {
        append_video_segment_index(&index_path, entry).map_err(|err| err.to_string())?;
    }
    Ok(())
}

fn request_ffmpeg_stop(child: &mut Child) -> Result<(), std::io::Error> {
    if let Some(stdin) = child.stdin.as_mut() {
        request_ffmpeg_stop_via_stdin(stdin)?;
    }

    let started_wait = Instant::now();
    while started_wait.elapsed() < Duration::from_secs(5) {
        if child.try_wait()?.is_some() {
            return Ok(());
        }
        thread::sleep(Duration::from_millis(100));
    }

    child.kill()?;
    let _ = child.wait();
    Ok(())
}

fn request_ffmpeg_stop_via_stdin(stdin: &mut ChildStdin) -> Result<(), std::io::Error> {
    use std::io::Write;

    stdin.write_all(b"q\n")?;
    stdin.flush()
}
