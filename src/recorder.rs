use crate::{
    burn_in::{burn_timestamp_overlay, timestamp_overlay_bounds},
    capture::{CaptureSource, CapturedFrame},
    config::RecorderConfig,
    diff::{DiffEngine, DiffError, PatchRegion},
    frame::Frame,
    session::{SessionLayout, SessionState},
    storage::{SessionDimensions, Storage, StorageError},
};
use serde::{Deserialize, Serialize};
use std::path::Path;
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};
use std::time::Duration;

const CONSERVATIVE_SAMPLE_GRID: usize = 8;
const BALANCED_SAMPLE_GRID: usize = 12;
const DETAILED_SAMPLE_GRID: usize = 16;

fn timestamp_second_changed(previous_timestamp_ms: u64, current_timestamp_ms: u64) -> bool {
    previous_timestamp_ms / 1_000 != current_timestamp_ms / 1_000
}

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct RecordingStats {
    pub frames_seen: u64,
    pub identical_frames_skipped: u64,
    pub sampled_precheck_skipped: u64,
    pub diff_runs: u64,
    pub patch_frames_written: u64,
    pub patch_regions_written: u64,
    pub keyframes_written: u64,
    pub started_at: u64,
    pub finished_at: u64,
}

impl RecordingStats {
    pub fn duration_ms(&self) -> u64 {
        self.finished_at.saturating_sub(self.started_at)
    }

    pub fn summary_line(&self) -> String {
        format!(
            "frames={} duration_ms={} identical_skips={} sampled_skips={} diff_runs={} patch_frames={} patch_regions={} keyframes={}",
            self.frames_seen,
            self.duration_ms(),
            self.identical_frames_skipped,
            self.sampled_precheck_skipped,
            self.diff_runs,
            self.patch_frames_written,
            self.patch_regions_written,
            self.keyframes_written
        )
    }
}

#[derive(Debug)]
pub enum RecorderError {
    NoFrames,
    Diff(DiffError),
    Storage(StorageError),
}

impl std::fmt::Display for RecorderError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NoFrames => write!(f, "capture produced no frames"),
            Self::Diff(err) => write!(f, "diff error: {err}"),
            Self::Storage(err) => write!(f, "{err}"),
        }
    }
}

impl std::error::Error for RecorderError {}

impl From<DiffError> for RecorderError {
    fn from(err: DiffError) -> Self {
        Self::Diff(err)
    }
}

impl From<StorageError> for RecorderError {
    fn from(err: StorageError) -> Self {
        Self::Storage(err)
    }
}

#[derive(Debug)]
pub enum RecordCommandError {
    CaptureUnavailable(String),
    Recorder(RecorderError),
}

impl std::fmt::Display for RecordCommandError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::CaptureUnavailable(message) => write!(f, "{message}"),
            Self::Recorder(err) => write!(f, "{err}"),
        }
    }
}

impl std::error::Error for RecordCommandError {}

impl From<RecorderError> for RecordCommandError {
    fn from(err: RecorderError) -> Self {
        Self::Recorder(err)
    }
}

pub fn record_command(
    config: RecorderConfig,
    session_id: impl Into<String>,
) -> Result<Storage, RecordCommandError> {
    let (storage, _) = record_command_with_stats(config, session_id)?;
    Ok(storage)
}

pub fn record_command_with_stats(
    config: RecorderConfig,
    session_id: impl Into<String>,
) -> Result<(Storage, RecordingStats), RecordCommandError> {
    let session_id = session_id.into();
    let stop_requested = Arc::new(AtomicBool::new(false));
    let stop_for_handler = Arc::clone(&stop_requested);
    ctrlc::set_handler(move || {
        stop_for_handler.store(true, Ordering::SeqCst);
    })
    .map_err(|err| RecordCommandError::CaptureUnavailable(err.to_string()))?;
    let session_layout = SessionLayout::new(&config.output_dir, &session_id);
    let stop_signal_path = session_layout.stop_signal_path().to_path_buf();

    record_command_with_stop_and_stats(config, session_id, move || {
        stop_requested.load(Ordering::SeqCst) || stop_signal_requested(&stop_signal_path)
    })
}

pub fn record_command_with_stop<F>(
    config: RecorderConfig,
    session_id: impl Into<String>,
    should_stop: F,
) -> Result<Storage, RecordCommandError>
where
    F: FnMut() -> bool,
{
    let (storage, _) = record_command_with_stop_and_stats(config, session_id, should_stop)?;
    Ok(storage)
}

pub fn record_command_with_stop_and_stats<F>(
    config: RecorderConfig,
    session_id: impl Into<String>,
    should_stop: F,
) -> Result<(Storage, RecordingStats), RecordCommandError>
where
    F: FnMut() -> bool,
{
    let session_id = session_id.into();
    #[cfg(target_os = "windows")]
    {
        use crate::capture::windows::{BackendKind, WindowsCapture};

        let capture = WindowsCapture::with_scale(
            BackendKind::PrimaryDisplayOnly,
            config.sampling_interval_ms,
            config.working_scale,
        )
        .map_err(|err| RecordCommandError::CaptureUnavailable(err.to_string()))?;
        let recorder = Recorder::new(config, session_id, capture);
        return recorder
            .run_until_with_stats(should_stop)
            .map_err(RecordCommandError::Recorder);
    }
    #[cfg(not(target_os = "windows"))]
    {
        let _ = config;
        let _ = session_id;
        let _ = should_stop;
        Err(RecordCommandError::CaptureUnavailable(
            "recording is only supported on Windows".to_string(),
        ))
    }
}

pub fn stop_signal_requested(path: &Path) -> bool {
    path.exists()
}

pub fn pause_signal_requested(path: &Path) -> bool {
    path.exists()
}

#[derive(Debug)]
pub struct Recorder<C: CaptureSource> {
    config: RecorderConfig,
    session_id: String,
    capture: C,
}

impl<C: CaptureSource> Recorder<C> {
    pub fn new(config: RecorderConfig, session_id: impl Into<String>, capture: C) -> Self {
        Self {
            config,
            session_id: session_id.into(),
            capture,
        }
    }

    pub fn run(self) -> Result<Storage, RecorderError> {
        self.run_until(|| false)
    }

    pub fn run_with_stats(self) -> Result<(Storage, RecordingStats), RecorderError> {
        self.run_until_with_stats(|| false)
    }

    pub fn run_until<F>(self, mut should_stop: F) -> Result<Storage, RecorderError>
    where
        F: FnMut() -> bool,
    {
        let (storage, _) = self.run_until_with_stats(&mut should_stop)?;
        Ok(storage)
    }

    pub fn run_until_with_stats<F>(
        mut self,
        mut should_stop: F,
    ) -> Result<(Storage, RecordingStats), RecorderError>
    where
        F: FnMut() -> bool,
    {
        let mut first = self.capture.next_frame().ok_or(RecorderError::NoFrames)?;
        if self.config.burn_in_enabled {
            burn_timestamp_overlay(&mut first.frame, first.timestamp_ms);
        }
        let mut diff_engine = DiffEngine::new(&self.config);
        let mut storage = self.start_storage(&first)?;
        let sampled_precheck_threshold = self.config.thresholds().precheck_threshold;
        let sample_grid = sample_grid_size(&self.config);
        let mut stats = RecordingStats {
            frames_seen: 1,
            keyframes_written: 1,
            started_at: first.timestamp_ms,
            finished_at: first.timestamp_ms,
            ..RecordingStats::default()
        };
        storage.write_status(SessionState::Running, &stats)?;

        let mut persisted_frame = first.frame;
        let mut last_timestamp = first.timestamp_ms;
        let mut last_keyframe_ms = last_timestamp;

        let keyframe_interval_ms = self.config.keyframe_interval_ms;
        let pause_poll_interval = Duration::from_millis(self.config.sampling_interval_ms.max(1));
        let session_layout = SessionLayout::new(&self.config.output_dir, &self.session_id);
        while !should_stop() {
            if pause_signal_requested(session_layout.pause_signal_path()) {
                storage.write_status(SessionState::Paused, &stats)?;
                std::thread::sleep(pause_poll_interval);
                continue;
            }

            let Some(mut frame) = self.capture.next_frame() else {
                break;
            };
            if self.config.burn_in_enabled {
                burn_timestamp_overlay(&mut frame.frame, frame.timestamp_ms);
            }
            stats.frames_seen += 1;

            if persisted_frame.as_rgba() != frame.frame.as_rgba() {
                let burn_in_second_changed = self.config.burn_in_enabled
                    && timestamp_second_changed(last_timestamp, frame.timestamp_ms);
                let sampled_difference = persisted_frame
                    .sampled_difference_ratio(&frame.frame, sample_grid, sample_grid);

                if burn_in_second_changed || sampled_difference >= sampled_precheck_threshold {
                    stats.diff_runs += 1;
                    let mut diff = diff_engine.diff(&persisted_frame, &frame.frame)?;
                    if burn_in_second_changed {
                        append_burn_in_overlay_patch_if_needed(&mut diff.patches, &frame.frame, frame.timestamp_ms);
                    }
                    if !diff.patches.is_empty() {
                        stats.patch_frames_written += 1;
                        stats.patch_regions_written += diff.patches.len() as u64;
                        storage.write_patches(frame.timestamp_ms, &diff.patches)?;
                        apply_patches_to_frame(&mut persisted_frame, &diff.patches);
                    }
                } else {
                    stats.sampled_precheck_skipped += 1;
                }
            } else {
                stats.identical_frames_skipped += 1;
            }

            if frame.timestamp_ms.saturating_sub(last_keyframe_ms) >= keyframe_interval_ms {
                let payload = full_frame_bytes(&frame);
                storage.write_keyframe(frame.timestamp_ms, &payload)?;
                last_keyframe_ms = frame.timestamp_ms;
                stats.keyframes_written += 1;
                persisted_frame = frame.frame.clone();
            }

            last_timestamp = frame.timestamp_ms;
            stats.finished_at = last_timestamp;
            storage.write_status(SessionState::Running, &stats)?;
        }

        stats.finished_at = last_timestamp;
        storage.write_status(SessionState::Stopped, &stats)?;
        storage.finalize_session(last_timestamp)?;
        Ok((storage, stats))
    }

    fn start_storage(&self, first: &CapturedFrame) -> Result<Storage, RecorderError> {
        let dimensions = self.capture.dimensions();
        let mut storage = Storage::start_session(
            self.config.clone(),
            &self.session_id,
            first.timestamp_ms,
            SessionDimensions {
                display_width: dimensions.display_width,
                display_height: dimensions.display_height,
                working_width: dimensions.working_width,
                working_height: dimensions.working_height,
            },
        )?;

        let payload = full_frame_bytes(first);
        storage.write_keyframe(first.timestamp_ms, &payload)?;
        Ok(storage)
    }
}

fn full_frame_bytes(frame: &CapturedFrame) -> Vec<u8> {
    frame
        .frame
        .copy_region_rgba(0, 0, frame.frame.width(), frame.frame.height())
}

fn sample_grid_size(config: &RecorderConfig) -> usize {
    match config.sensitivity_mode {
        crate::config::SensitivityMode::Conservative => CONSERVATIVE_SAMPLE_GRID,
        crate::config::SensitivityMode::Balanced => BALANCED_SAMPLE_GRID,
        crate::config::SensitivityMode::Detailed => DETAILED_SAMPLE_GRID,
    }
}

fn append_burn_in_overlay_patch_if_needed(
    patches: &mut Vec<PatchRegion>,
    frame: &Frame,
    timestamp_ms: u64,
) {
    let timestamp_text = crate::burn_in::format_timestamp_to_seconds(timestamp_ms);
    let Some(bounds) = timestamp_overlay_bounds(frame, &timestamp_text) else {
        return;
    };
    let overlay_patch = PatchRegion {
        x: bounds.x as u32,
        y: bounds.y as u32,
        width: bounds.width as u32,
        height: bounds.height as u32,
        data: frame.copy_region_rgba(bounds.x, bounds.y, bounds.width, bounds.height),
    };

    if patches.iter().any(|patch| regions_overlap(patch, &overlay_patch)) {
        return;
    }

    patches.push(overlay_patch);
}

fn regions_overlap(left: &PatchRegion, right: &PatchRegion) -> bool {
    let left_x2 = left.x + left.width;
    let left_y2 = left.y + left.height;
    let right_x2 = right.x + right.width;
    let right_y2 = right.y + right.height;

    left.x < right_x2 && right.x < left_x2 && left.y < right_y2 && right.y < left_y2
}

fn apply_patches_to_frame(frame: &mut Frame, patches: &[PatchRegion]) {
    for patch in patches {
        let mut cursor = 0usize;
        for y in patch.y as usize..(patch.y + patch.height) as usize {
            for x in patch.x as usize..(patch.x + patch.width) as usize {
                let rgba = [
                    patch.data[cursor],
                    patch.data[cursor + 1],
                    patch.data[cursor + 2],
                    patch.data[cursor + 3],
                ];
                frame.set_pixel(x, y, rgba);
                cursor += 4;
            }
        }
    }
}
