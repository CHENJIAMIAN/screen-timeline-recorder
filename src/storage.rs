use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::config::RecorderConfig;
use crate::diff::PatchRegion;
use crate::index::{
    IndexError, KeyframeIndexEntry, PatchIndexEntry, append_keyframe_index, append_patch_index,
};
use crate::logging::StructuredError;
use crate::recorder::RecordingStats;
use crate::session::{Manifest, SessionLayout, SessionState, SessionStatus};

const COALESCE_WINDOW_MS: u64 = 200;
const COMPRESSION_FORMAT: &str = "png";
const LEGACY_COMPRESSION_FORMAT: &str = "raw";
const PATCH_MAGIC: &[u8; 4] = b"STP1";

#[derive(Debug, Clone, Copy)]
pub struct SessionDimensions {
    pub display_width: u32,
    pub display_height: u32,
    pub working_width: u32,
    pub working_height: u32,
}

#[derive(Debug)]
pub enum StorageError {
    Structured(StructuredError),
    Index(IndexError),
    PngEncoding(png::EncodingError),
    PngDecoding(png::DecodingError),
}

impl std::fmt::Display for StorageError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Structured(err) => write!(f, "{}", err.message),
            Self::Index(err) => write!(f, "{err}"),
            Self::PngEncoding(err) => write!(f, "png encoding failed: {err}"),
            Self::PngDecoding(err) => write!(f, "png decoding failed: {err}"),
        }
    }
}

impl std::error::Error for StorageError {}

impl From<IndexError> for StorageError {
    fn from(err: IndexError) -> Self {
        Self::Index(err)
    }
}

impl From<png::EncodingError> for StorageError {
    fn from(err: png::EncodingError) -> Self {
        Self::PngEncoding(err)
    }
}

impl From<png::DecodingError> for StorageError {
    fn from(err: png::DecodingError) -> Self {
        Self::PngDecoding(err)
    }
}

#[derive(Debug, Clone)]
struct LastPatchInfo {
    timestamp_ms: u64,
    region: PatchRegion,
}

#[derive(Debug, Serialize, Deserialize)]
struct StoredPatchRegion {
    x: u32,
    y: u32,
    width: u32,
    height: u32,
    data: Vec<u8>,
}

#[derive(Debug)]
pub struct Storage {
    _config: RecorderConfig,
    layout: SessionLayout,
    manifest: Manifest,
    last_patch: Option<LastPatchInfo>,
    patch_sequence: u64,
}

impl Storage {
    pub fn start_session(
        config: RecorderConfig,
        session_id: &str,
        started_at: u64,
        dimensions: SessionDimensions,
    ) -> Result<Self, StorageError> {
        let layout = SessionLayout::new(&config.output_dir, session_id);
        layout
            .create_dirs()
            .map_err(|err| storage_io("create_session_dirs", layout.root(), err))?;

        let thresholds = config.thresholds();
        let manifest = Manifest {
            session_id: session_id.to_string(),
            started_at,
            finished_at: None,
            display_width: dimensions.display_width,
            display_height: dimensions.display_height,
            working_width: dimensions.working_width,
            working_height: dimensions.working_height,
            sampling_interval_ms: config.sampling_interval_ms,
            block_width: config.block_width,
            block_height: config.block_height,
            keyframe_interval_ms: config.keyframe_interval_ms,
            sensitivity_mode: format!("{:?}", config.sensitivity_mode).to_lowercase(),
            precheck_threshold: thresholds.precheck_threshold,
            block_difference_threshold: thresholds.block_difference_threshold,
            changed_pixel_ratio_threshold: thresholds.changed_pixel_ratio_threshold,
            stability_window: thresholds.stability_window,
            compression_format: COMPRESSION_FORMAT.to_string(),
            recorder_version: env!("CARGO_PKG_VERSION").to_string(),
            viewer_default_zoom: config.viewer_default_zoom,
            viewer_overlay_enabled_by_default: config.viewer_overlay_enabled_by_default,
            burn_in_enabled: config.burn_in_enabled,
            viewer_language: config.viewer_language,
        };

        let manifest_json = serde_json::to_string_pretty(&manifest)
            .map_err(|err| storage_json("serialize_manifest", Some(layout.manifest_path()), err))?;
        std::fs::write(layout.manifest_path(), manifest_json)
            .map_err(|err| storage_io("write_manifest", layout.manifest_path(), err))?;
        write_status_file(
            &layout,
            &SessionStatus {
                session_id: session_id.to_string(),
                state: SessionState::Running,
                recording: true,
                stats: RecordingStats {
                    started_at,
                    finished_at: started_at,
                    ..RecordingStats::default()
                },
            },
        )?;

        Ok(Self {
            _config: config,
            layout,
            manifest,
            last_patch: None,
            patch_sequence: 0,
        })
    }

    pub fn layout(&self) -> &SessionLayout {
        &self.layout
    }

    pub fn manifest(&self) -> &Manifest {
        &self.manifest
    }

    pub fn write_keyframe(
        &mut self,
        timestamp_ms: u64,
        data: &[u8],
    ) -> Result<PathBuf, StorageError> {
        let filename = format!("{timestamp_ms}.png");
        let path = self.layout.keyframes_dir().join(&filename);
        let encoded = encode_png_rgba(
            self.manifest.working_width,
            self.manifest.working_height,
            data,
        )?;
        std::fs::write(&path, encoded).map_err(|err| storage_io("write_keyframe", &path, err))?;

        let entry = KeyframeIndexEntry {
            timestamp_ms,
            path: format!("keyframes/{filename}"),
        };
        append_keyframe_index(self.layout.index_dir(), &entry)?;
        Ok(path)
    }

    pub fn write_patches(
        &mut self,
        timestamp_ms: u64,
        patches: &[PatchRegion],
    ) -> Result<Vec<PathBuf>, StorageError> {
        if patches.is_empty() {
            return Ok(Vec::new());
        }

        let mut written_paths = Vec::new();
        let previous_last_patch = self.last_patch.clone();
        let mut latest_written_patch: Option<LastPatchInfo> = None;
        for patch in patches {
            if should_coalesce(previous_last_patch.as_ref(), timestamp_ms, patch) {
                continue;
            }

            let sequence = self.patch_sequence;
            self.patch_sequence += 1;

            let filename = format!("{timestamp_ms}_{sequence}.stp");
            let path = self.layout.patches_dir().join(&filename);
            let payload = encode_patch_region(&StoredPatchRegion {
                x: patch.x,
                y: patch.y,
                width: patch.width,
                height: patch.height,
                data: patch.data.clone(),
            })?;
            std::fs::write(&path, payload).map_err(|err| storage_io("write_patch", &path, err))?;

            let entry = PatchIndexEntry {
                timestamp_ms,
                sequence,
                path: format!("patches/{filename}"),
            };
            append_patch_index(self.layout.index_dir(), &entry)?;

            latest_written_patch = Some(LastPatchInfo {
                timestamp_ms,
                region: patch.clone(),
            });

            written_paths.push(path);
        }

        if let Some(last_patch) = latest_written_patch {
            self.last_patch = Some(last_patch);
        }

        Ok(written_paths)
    }

    pub fn load_manifest(&self) -> Result<Manifest, StorageError> {
        Manifest::load(self.layout.manifest_path())
            .map_err(|err| storage_io("read_manifest", self.layout.manifest_path(), err))
    }

    pub fn write_status(
        &mut self,
        state: SessionState,
        stats: &RecordingStats,
    ) -> Result<(), StorageError> {
        write_status_file(
            &self.layout,
            &SessionStatus {
                session_id: self.manifest.session_id.clone(),
                state,
                recording: state != SessionState::Stopped,
                stats: stats.clone(),
            },
        )
    }

    pub fn finalize_session(&mut self, finished_at: u64) -> Result<(), StorageError> {
        self.manifest.finished_at = Some(finished_at);
        let manifest_json = serde_json::to_string_pretty(&self.manifest).map_err(|err| {
            storage_json("serialize_manifest", Some(self.layout.manifest_path()), err)
        })?;
        std::fs::write(self.layout.manifest_path(), manifest_json)
            .map_err(|err| storage_io("write_manifest", self.layout.manifest_path(), err))?;
        let existing_status = SessionStatus::load(self.layout.status_path())
            .map_err(|err| storage_io("read_status", self.layout.status_path(), err))?;
        self.write_status(
            SessionState::Stopped,
            &RecordingStats {
                started_at: existing_status
                    .stats
                    .started_at
                    .max(self.manifest.started_at),
                finished_at,
                ..existing_status.stats
            },
        )?;
        Ok(())
    }

    pub fn read_keyframe_bytes(&self, entry: &KeyframeIndexEntry) -> Result<Vec<u8>, StorageError> {
        read_keyframe_bytes(
            self.layout(),
            entry,
            &self.manifest.compression_format,
            self.manifest.working_width,
            self.manifest.working_height,
        )
    }

    pub fn read_patch_region(&self, entry: &PatchIndexEntry) -> Result<PatchRegion, StorageError> {
        read_patch_region(self.layout(), entry, &self.manifest.compression_format)
    }
}

pub fn read_keyframe_bytes(
    layout: &SessionLayout,
    entry: &KeyframeIndexEntry,
    compression_format: &str,
    width: u32,
    height: u32,
) -> Result<Vec<u8>, StorageError> {
    let path = layout.root().join(&entry.path);
    let payload = std::fs::read(&path).map_err(|err| storage_io("read_keyframe", &path, err))?;
    match compression_format {
        LEGACY_COMPRESSION_FORMAT => Ok(payload),
        COMPRESSION_FORMAT => decode_png_rgba(&payload, width, height),
        other => Err(storage_patch_format(
            &path,
            Box::leak(format!("unsupported compression format: {other}").into_boxed_str()),
        )),
    }
}

pub fn read_patch_region(
    layout: &SessionLayout,
    entry: &PatchIndexEntry,
    compression_format: &str,
) -> Result<PatchRegion, StorageError> {
    let path = layout.root().join(&entry.path);
    let payload = std::fs::read(&path).map_err(|err| storage_io("read_patch", &path, err))?;
    let stored = match compression_format {
        LEGACY_COMPRESSION_FORMAT => serde_json::from_slice(&payload)
            .map_err(|err| storage_json("deserialize_patch", Some(&path), err))?,
        COMPRESSION_FORMAT => decode_patch_region(&payload)
            .map_err(|message| storage_patch_format_owned(&path, message))?,
        other => {
            return Err(storage_patch_format_owned(
                &path,
                format!("unsupported compression format: {other}"),
            ));
        }
    };
    if stored.width == 0 || stored.height == 0 {
        return Err(storage_patch_format(
            &path,
            "patch dimensions must be non-zero",
        ));
    }

    Ok(PatchRegion {
        x: stored.x,
        y: stored.y,
        width: stored.width,
        height: stored.height,
        data: stored.data,
    })
}

fn encode_png_rgba(width: u32, height: u32, data: &[u8]) -> Result<Vec<u8>, StorageError> {
    let expected_len = width as usize * height as usize * 4;
    if data.len() != expected_len {
        return Err(storage_patch_format_owned(
            Path::new(""),
            format!(
                "rgba buffer length {} does not match expected {} for {}x{}",
                data.len(),
                expected_len,
                width,
                height
            ),
        ));
    }

    let mut encoded = Vec::new();
    let mut encoder = png::Encoder::new(&mut encoded, width, height);
    encoder.set_color(png::ColorType::Rgba);
    encoder.set_depth(png::BitDepth::Eight);
    {
        let mut writer = encoder.write_header()?;
        writer.write_image_data(data)?;
    }
    Ok(encoded)
}

fn decode_png_rgba(payload: &[u8], width: u32, height: u32) -> Result<Vec<u8>, StorageError> {
    let decoder = png::Decoder::new(std::io::Cursor::new(payload));
    let mut reader = decoder.read_info()?;
    let mut buffer = vec![0; reader.output_buffer_size()];
    let info = reader.next_frame(&mut buffer)?;
    if info.width != width || info.height != height {
        return Err(storage_patch_format_owned(
            Path::new(""),
            format!(
                "decoded png size {}x{} does not match expected {}x{}",
                info.width, info.height, width, height
            ),
        ));
    }
    Ok(buffer[..info.buffer_size()].to_vec())
}

fn encode_patch_region(stored: &StoredPatchRegion) -> Result<Vec<u8>, StorageError> {
    let mut encoded = Vec::with_capacity(20);
    encoded.extend_from_slice(PATCH_MAGIC);
    encoded.extend_from_slice(&stored.x.to_le_bytes());
    encoded.extend_from_slice(&stored.y.to_le_bytes());
    encoded.extend_from_slice(&stored.width.to_le_bytes());
    encoded.extend_from_slice(&stored.height.to_le_bytes());
    encoded.extend_from_slice(&encode_png_rgba(stored.width, stored.height, &stored.data)?);
    Ok(encoded)
}

fn decode_patch_region(payload: &[u8]) -> Result<StoredPatchRegion, String> {
    if payload.len() < 20 {
        return Err("patch payload is too short".to_string());
    }
    if &payload[..4] != PATCH_MAGIC {
        return Err("patch payload header is invalid".to_string());
    }

    let x = u32::from_le_bytes(payload[4..8].try_into().expect("x bytes"));
    let y = u32::from_le_bytes(payload[8..12].try_into().expect("y bytes"));
    let width = u32::from_le_bytes(payload[12..16].try_into().expect("width bytes"));
    let height = u32::from_le_bytes(payload[16..20].try_into().expect("height bytes"));
    let data = decode_png_rgba(&payload[20..], width, height).map_err(|err| err.to_string())?;

    Ok(StoredPatchRegion {
        x,
        y,
        width,
        height,
        data,
    })
}

fn should_coalesce(
    previous_last_patch: Option<&LastPatchInfo>,
    timestamp_ms: u64,
    patch: &PatchRegion,
) -> bool {
    match previous_last_patch {
        Some(last)
            if last.region.matches_region(patch)
                && timestamp_ms.saturating_sub(last.timestamp_ms) <= COALESCE_WINDOW_MS =>
        {
            true
        }
        _ => false,
    }
}

fn storage_io(operation: &'static str, path: &Path, err: std::io::Error) -> StorageError {
    StorageError::Structured(StructuredError::from_io(operation, path.to_path_buf(), err))
}

fn storage_json(
    operation: &'static str,
    path: Option<&Path>,
    err: serde_json::Error,
) -> StorageError {
    StorageError::Structured(StructuredError::from_json(
        operation,
        path.map(|value| value.to_path_buf()),
        err,
    ))
}

fn storage_patch_format(path: &Path, message: &'static str) -> StorageError {
    StorageError::Structured(StructuredError::new(
        "deserialize_patch",
        Some(path.to_path_buf()),
        "patch_format",
        message,
    ))
}

fn storage_patch_format_owned(path: &Path, message: String) -> StorageError {
    StorageError::Structured(StructuredError::new(
        "deserialize_patch",
        Some(path.to_path_buf()),
        "patch_format",
        Box::leak(message.into_boxed_str()),
    ))
}

fn write_status_file(layout: &SessionLayout, status: &SessionStatus) -> Result<(), StorageError> {
    let payload = serde_json::to_string_pretty(status)
        .map_err(|err| storage_json("serialize_status", Some(layout.status_path()), err))?;
    std::fs::write(layout.status_path(), payload)
        .map_err(|err| storage_io("write_status", layout.status_path(), err))?;
    Ok(())
}
