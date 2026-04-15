use std::path::Path;

use crate::{
    diff::PatchRegion,
    frame::Frame,
    index::{IndexError, nearest_keyframe, patch_entries_between},
    logging::warn,
    session::{Manifest, SessionLayout},
    storage::{StorageError, read_keyframe_bytes, read_patch_region},
};

#[derive(Debug)]
pub enum ReconstructError {
    Io(std::io::Error),
    Storage(StorageError),
    Index(IndexError),
    MissingKeyframe(u64),
    InvalidKeyframeBuffer,
}

impl std::fmt::Display for ReconstructError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Io(err) => write!(f, "reconstruction io error: {err}"),
            Self::Storage(err) => write!(f, "{err}"),
            Self::Index(err) => write!(f, "{err}"),
            Self::MissingKeyframe(timestamp_ms) => {
                write!(f, "no keyframe available for timestamp {timestamp_ms}")
            }
            Self::InvalidKeyframeBuffer => write!(f, "invalid keyframe buffer"),
        }
    }
}

impl std::error::Error for ReconstructError {}

impl From<std::io::Error> for ReconstructError {
    fn from(err: std::io::Error) -> Self {
        Self::Io(err)
    }
}

impl From<StorageError> for ReconstructError {
    fn from(err: StorageError) -> Self {
        Self::Storage(err)
    }
}

impl From<IndexError> for ReconstructError {
    fn from(err: IndexError) -> Self {
        Self::Index(err)
    }
}

pub struct Reconstructor {
    layout: SessionLayout,
    manifest: Manifest,
}

impl Reconstructor {
    pub fn open(output_dir: &Path, session_id: &str) -> Result<Self, ReconstructError> {
        let layout = SessionLayout::new(output_dir, session_id);
        let manifest = Manifest::load(layout.manifest_path())?;
        Ok(Self { layout, manifest })
    }

    pub fn reconstruct_at(&self, timestamp_ms: u64) -> Result<Frame, ReconstructError> {
        let keyframe = nearest_keyframe(self.layout.index_dir(), timestamp_ms)?
            .ok_or(ReconstructError::MissingKeyframe(timestamp_ms))?;
        let keyframe_bytes = read_keyframe_bytes(
            &self.layout,
            &keyframe,
            &self.manifest.compression_format,
            self.manifest.working_width,
            self.manifest.working_height,
        )?;
        let width = self.manifest.working_width as usize;
        let height = self.manifest.working_height as usize;
        if keyframe_bytes.len() != width * height * 4 {
            return Err(ReconstructError::InvalidKeyframeBuffer);
        }

        let mut frame = Frame::from_rgba(width, height, keyframe_bytes);
        let patches = patch_entries_between(
            self.layout.index_dir(),
            keyframe.timestamp_ms + 1,
            timestamp_ms,
        )?;
        for patch_entry in patches {
            match read_patch_region(
                &self.layout,
                &patch_entry,
                &self.manifest.compression_format,
            ) {
                Ok(patch) => apply_patch(&mut frame, &patch),
                Err(err) => {
                    if let StorageError::Structured(structured) = &err {
                        warn("skipping patch due to read error", Some(structured));
                        continue;
                    }
                    return Err(ReconstructError::Storage(err));
                }
            }
        }

        Ok(frame)
    }
}

fn apply_patch(frame: &mut Frame, patch: &PatchRegion) {
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
