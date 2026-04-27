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
    let mut logical_finished_at = started_at;
    let mut next_segment_sequence = 0;
    let mut all_segment_entries = Vec::new();

    loop {
        wait_until_resumable_or_stopped(&layout);
        if layout.stop_signal_path().exists() {
            break;
        }

        write_status(
            &layout,
            SessionState::Running,
            RecordingStats {
                started_at,
                finished_at: logical_finished_at,
                ..RecordingStats::default()
            },
        )?;

        let run_started_at = current_timestamp_ms();
        let args = build_ffmpeg_segment_args(
            &config,
            &layout,
            display_width,
            display_height,
            video_width,
            video_height,
            DEFAULT_SEGMENT_DURATION_SECS,
            next_segment_sequence,
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

        let ended_by_pause = loop {
            if layout.stop_signal_path().exists() {
                request_ffmpeg_stop(&mut child).map_err(|err| err.to_string())?;
                break false;
            }

            if layout.pause_signal_path().exists() {
                request_ffmpeg_stop(&mut child).map_err(|err| err.to_string())?;
                break true;
            }

            if let Some(status) = child.try_wait().map_err(|err| err.to_string())? {
                break classify_ffmpeg_exit(status.success(), layout.pause_signal_path().exists())?;
            }

            thread::sleep(Duration::from_millis(250));
        };

        let run_finished_at = current_timestamp_ms();
        let run_elapsed_ms = run_finished_at.saturating_sub(run_started_at);
        let new_segment_files = collect_run_segment_files(&layout, next_segment_sequence)
            .map_err(|err| err.to_string())?;
        let run_entries = build_segment_entries_for_run(
            logical_finished_at,
            run_elapsed_ms,
            manifest.segment_duration_ms,
            next_segment_sequence,
            &new_segment_files,
        );
        next_segment_sequence += run_entries.len() as u64;
        if let Some(last_entry) = run_entries.last() {
            logical_finished_at = last_entry.finished_at.unwrap_or(logical_finished_at);
        }
        all_segment_entries.extend(run_entries);
        write_video_segment_index(&layout, &all_segment_entries)?;

        if ended_by_pause {
            write_status(
                &layout,
                SessionState::Paused,
                RecordingStats {
                    started_at,
                    finished_at: logical_finished_at,
                    ..RecordingStats::default()
                },
            )?;
            continue;
        }

        break;
    }

    write_status(
        &layout,
        SessionState::Stopped,
        RecordingStats {
            started_at,
            finished_at: logical_finished_at,
            ..RecordingStats::default()
        },
    )?;
    finalize_manifest(&layout, logical_finished_at)?;
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
    bundled_candidates
        .into_iter()
        .find(|path| path.is_file())
        .or_else(|| resolve_dev_repo_ffmpeg_path(exe_dir))
}

fn resolve_dev_repo_ffmpeg_path(exe_dir: &Path) -> Option<PathBuf> {
    if !looks_like_dev_target_dir(exe_dir) {
        return None;
    }

    let repo_candidate = exe_dir
        .ancestors()
        .nth(2)
        .map(|repo_root| repo_root.join("tools").join("ffmpeg").join("ffmpeg.exe"))?;

    repo_candidate.is_file().then_some(repo_candidate)
}

fn looks_like_dev_target_dir(exe_dir: &Path) -> bool {
    let current = exe_dir.file_name().and_then(|name| name.to_str());
    let parent = exe_dir
        .parent()
        .and_then(|path| path.file_name())
        .and_then(|name| name.to_str());
    matches!(current, Some("debug") | Some("release")) && matches!(parent, Some("target"))
}

pub fn build_ffmpeg_segment_args(
    config: &RecorderConfig,
    layout: &SessionLayout,
    display_width: u32,
    display_height: u32,
    video_width: u32,
    video_height: u32,
    segment_duration_secs: u64,
    start_number: u64,
    _ffmpeg_path: &Path,
) -> Vec<String> {
    let framerate = ((1_000f32 / config.sampling_interval_ms.max(1) as f32).floor() as u64).max(1);
    let segment_duration_secs = segment_duration_secs.max(DEFAULT_SEGMENT_DURATION_SECS);
    let video_filter = build_video_filter(config, display_width, display_height, video_width, video_height);
    let segment_pattern = layout
        .segments_dir()
        .join(format!("run-{start_number:06}-%06d.mp4"));

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

pub fn build_segment_entries_for_run(
    logical_started_at: u64,
    run_elapsed_ms: u64,
    segment_duration_ms: u64,
    start_sequence: u64,
    segment_files: &[(String, u64)],
) -> Vec<VideoSegmentEntry> {
    let mut entries = Vec::new();
    let mut next_started_at = logical_started_at;
    let full_segments = segment_files.len().saturating_sub(1) as u64;
    let consumed_by_full_segments = full_segments.saturating_mul(segment_duration_ms);
    let trailing_segment_duration = run_elapsed_ms
        .saturating_sub(consumed_by_full_segments)
        .min(segment_duration_ms);

    for (index, (file_name, bytes)) in segment_files.iter().enumerate() {
        let duration_ms = if index + 1 == segment_files.len() {
            trailing_segment_duration
        } else {
            segment_duration_ms
        };
        let finished_at = next_started_at.saturating_add(duration_ms);
        entries.push(VideoSegmentEntry {
            sequence: start_sequence + index as u64,
            started_at: next_started_at,
            finished_at: Some(finished_at),
            relative_path: format!("segments/{file_name}"),
            bytes: *bytes,
        });
        next_started_at = finished_at;
    }

    entries
}

pub fn classify_ffmpeg_exit(success: bool, pause_requested: bool) -> Result<bool, String> {
    if pause_requested {
        return Ok(true);
    }
    if success {
        return Ok(false);
    }
    Err("ffmpeg exited before a pause or stop request".to_string())
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

fn collect_run_segment_files(
    layout: &SessionLayout,
    start_sequence: u64,
) -> Result<Vec<(String, u64)>, std::io::Error> {
    let run_prefix = format!("run-{start_sequence:06}-");
    let mut files: Vec<_> = fs::read_dir(layout.segments_dir())?
        .filter_map(|entry| entry.ok())
        .filter(|entry| entry.file_type().map(|kind| kind.is_file()).unwrap_or(false))
        .filter_map(|entry| {
            let file_name = entry.file_name().to_string_lossy().to_string();
            let local_sequence = parse_run_segment_sequence(&file_name, &run_prefix)?;
            Some((local_sequence, entry))
        })
        .collect();
    files.sort_by_key(|(sequence, _)| *sequence);

    let mut collected = Vec::new();
    for (offset, entry) in files {
        let global_sequence = start_sequence + offset;
        let final_file_name = format!("{global_sequence:06}.mp4");
        let final_path = layout.segments_dir().join(&final_file_name);
        if final_path.exists() {
            fs::remove_file(&final_path)?;
        }
        fs::rename(entry.path(), &final_path)?;
        let metadata = fs::metadata(&final_path)?;
        collected.push((final_file_name, metadata.len()));
    }
    Ok(collected)
}

fn parse_run_segment_sequence(file_name: &str, run_prefix: &str) -> Option<u64> {
    let rest = file_name.strip_prefix(run_prefix)?;
    let stem = Path::new(rest).file_stem()?.to_str()?;
    stem.parse().ok()
}

#[allow(dead_code)]
fn collect_segment_files_from_sequence(
    layout: &SessionLayout,
    start_sequence: u64,
) -> Result<Vec<(String, u64)>, std::io::Error> {
    let mut files: Vec<_> = fs::read_dir(layout.segments_dir())?
        .filter_map(|entry| entry.ok())
        .filter(|entry| entry.file_type().map(|kind| kind.is_file()).unwrap_or(false))
        .filter_map(|entry| {
            let file_name = entry.file_name().to_string_lossy().to_string();
            let sequence = parse_segment_sequence(&file_name)?;
            (sequence >= start_sequence).then_some((sequence, entry))
        })
        .collect();
    files.sort_by_key(|(sequence, _)| *sequence);

    let mut collected = Vec::new();
    for (_, entry) in files {
        let metadata = entry.metadata()?;
        collected.push((entry.file_name().to_string_lossy().to_string(), metadata.len()));
    }
    Ok(collected)
}

fn parse_segment_sequence(file_name: &str) -> Option<u64> {
    let stem = Path::new(file_name).file_stem()?.to_str()?;
    stem.parse().ok()
}

fn wait_until_resumable_or_stopped(layout: &SessionLayout) {
    while layout.pause_signal_path().exists() && !layout.stop_signal_path().exists() {
        thread::sleep(Duration::from_millis(250));
    }
}

fn request_ffmpeg_stop(child: &mut Child) -> Result<(), std::io::Error> {
    if child.try_wait()?.is_some() {
        return Ok(());
    }

    if let Some(stdin) = child.stdin.as_mut() {
        if let Err(err) = request_ffmpeg_stop_via_stdin(stdin)
            && !is_ignorable_ffmpeg_stop_error(&err)
        {
            return Err(err);
        }
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

pub fn is_ignorable_ffmpeg_stop_error(error: &std::io::Error) -> bool {
    matches!(
        error.kind(),
        std::io::ErrorKind::BrokenPipe | std::io::ErrorKind::InvalidInput
    )
}
