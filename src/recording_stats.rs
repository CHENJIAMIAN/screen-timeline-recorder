use serde::{Deserialize, Serialize};

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
