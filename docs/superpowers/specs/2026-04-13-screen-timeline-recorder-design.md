# Screen Timeline Recorder Design

## Summary

Build a Windows-first tool that runs continuously through the day and records a low-cost, low-storage history of screen state changes. The tool is not a traditional video recorder. It stores a time-indexed stream of visual deltas so the user can later scrub a timeline and inspect what changed during the day.

Primary goals:

- Start once and run all day with minimal user interaction.
- Minimize CPU, memory, and disk usage.
- Avoid writing redundant data when the screen is unchanged.
- Prefer storing only changed regions instead of full frames.
- Default to low visual fidelity, with configurable sensitivity and quality.
- Support local timeline review instead of exporting a standard video.

Non-goals for v1:

- Audio capture
- OCR or semantic activity classification
- Cloud sync
- Multi-monitor capture
- Standard video export

## User Experience

The user launches the recorder once at the beginning of the day. The recorder runs in the background and continuously inspects the primary display. When meaningful visual changes occur, it persists compact patch records. Later, the user opens a local viewer and scrubs through a timeline to inspect the reconstructed screen state at chosen timestamps.

The user can tune:

- Sampling interval
- Sensitivity profile
- Keyframe interval
- Block size
- Output directory
- Viewer defaults such as zoom and change overlay visibility

## Architecture

The system is split into four modules.

### 1. Capture

Responsibilities:

- Acquire screen images from the primary display.
- Downscale frames for quick pre-checks.
- Provide full-resolution or working-resolution frames to the diff stage only when needed.

Design decisions:

- Windows-only in v1.
- Use Windows Graphics Capture if practical in the chosen implementation language.
- Sampling is timer-driven rather than real-time video streaming.
- Default sampling targets low cadence, for example 500 ms to 2000 ms.

### 2. Diff

Responsibilities:

- Detect whether meaningful change occurred.
- Identify changed regions by block.
- Filter transient or tiny changes that are likely noise.

Pipeline:

1. Low-resolution pre-check compares the current frame against the previous frame.
2. If overall change is below threshold, skip storage.
3. If meaningful change is detected, run block-level comparison on the working frame.
4. Apply thresholds for per-block difference magnitude and changed-pixel ratio.
5. Apply temporal stability checks so one-off blips can be ignored by default.

Sensitivity modes:

- `conservative`: aggressively filters small changes and records the least data
- `balanced`: default mode for normal use
- `detailed`: captures more subtle changes at higher storage cost

### 3. Store

Responsibilities:

- Persist a session manifest.
- Persist periodic keyframes.
- Persist patch records for changed blocks.
- Maintain lookup indexes for fast timeline access.

Session layout:

```text
sessions/
  2026-04-13/
    manifest.json
    keyframes/
    patches/
    index/
```

Stored data:

- `manifest.json`
  - Session metadata, screen geometry, capture parameters, thresholds, and timestamps
- `keyframes/`
  - Periodic baseline images to bound reconstruction cost
- `patches/`
  - Changed blocks with timestamps, coordinates, dimensions, and encoded image payloads
- `index/`
  - Time-based lookup structures to jump quickly to nearby keyframes and patch segments

Storage rules:

- Persist nothing if no meaningful change occurs.
- Save only changed blocks for normal patch events.
- Save a periodic keyframe, even if changes are sparse, to cap replay cost.
- Merge rapid repeated writes to the same area when possible to reduce fragmentation.

Compression strategy:

- V1 uses simple, stable per-block image compression rather than an advanced custom codec.
- Lossless block storage is preferred first for implementation safety.
- Compression can evolve later if profiling shows strong benefit.

## Viewer

The viewer is a local timeline browser, not a media player.

Capabilities for v1:

- Open a recorded session
- Scrub a time slider
- Reconstruct and display screen state at the selected timestamp
- Jump by coarse time increments
- Zoom into the reconstructed image
- Optionally overlay the regions that changed at the current step

The viewer may be implemented as a local web app backed by the recorder process or by direct file reads if the chosen stack allows it cleanly.

## Data Model

### Manifest

Suggested fields:

- Session ID
- Start and end timestamps
- Display width and height
- Working resolution width and height
- Sampling interval
- Diff block size
- Sensitivity mode
- Threshold values
- Keyframe interval
- Compression format
- Recorder version

### Keyframe

Suggested fields:

- Timestamp
- Encoded image path
- Source dimensions

### Patch record

Suggested fields:

- Timestamp
- Sequence number
- Block origin x and y
- Block width and height
- Working resolution dimensions
- Encoded patch payload path or blob reference
- Diff metrics used for debugging or tuning

### Index

Suggested fields:

- Time to nearest keyframe mapping
- Patch segment offsets or file groupings
- Optional summary counts per minute for thumbnail generation

## Change Detection Strategy

The recorder is optimized to ignore useless churn.

Rules:

- Skip the frame entirely if low-resolution pre-check shows negligible change.
- Ignore blocks whose change magnitude stays below threshold.
- Ignore tiny changed-pixel ratios by default.
- Ignore a single transient block change unless it exceeds a stronger threshold.
- Allow multiple modes so the user can trade sensitivity for storage cost.

Likely noise sources:

- Caret blinking
- Minor tray icon animation
- Clock changes
- Small hover effects
- Transient toasts

Expected meaningful changes:

- Window switches
- Document edits
- Code changes
- Tab changes
- Spreadsheet edits
- Application navigation

## Performance Strategy

V1 is designed around bounded work.

Approach:

- Low-frequency polling instead of high-frame-rate capture
- Low-resolution pre-check before any deeper diff
- Fixed-size block diff for predictable cost
- No disk writes on unchanged frames
- Periodic keyframes to keep viewer reconstruction bounded
- Configurable defaults that favor low fidelity

Important tradeoff:

The system is optimized for recoverable work history, not smooth playback. It intentionally sacrifices motion fidelity for lower compute and storage overhead.

## Technical Approaches Considered

### Option A: Rust core plus local web viewer

Pros:

- Strong control over memory and CPU behavior
- Well suited for a long-running background recorder
- Good fit for custom diff and storage pipeline
- Easy to ship as a single executable plus static viewer assets

Cons:

- Higher implementation complexity
- Windows capture integration requires more low-level work

### Option B: .NET desktop app

Pros:

- Fast Windows-focused development
- Good UI options
- Easier integration with some Windows APIs

Cons:

- Less attractive if the diff and storage pipeline needs tight low-overhead control
- GUI-first structure may slow down the core recorder architecture

### Option C: Python prototype

Pros:

- Fast experimentation

Cons:

- Weaker fit for always-on long-running capture
- Higher risk on performance and packaging

Recommendation:

Choose Option A. Use Rust for recorder and reconstruction core, and a minimal local web viewer for session browsing.

## Error Handling

V1 should handle:

- Capture interruptions
- Session directory creation failures
- Insufficient disk space
- Corrupt patch files
- Graceful shutdown with manifest finalization

Behavior:

- Write structured logs for debugging
- Keep partial sessions readable where possible
- Fail a patch write without destroying the full session
- Surface health state in the viewer or CLI output

## Testing Strategy

V1 needs automated coverage around the deterministic parts of the system.

Test focus:

- Block diff correctness
- Threshold and noise filtering behavior
- Session layout and manifest writing
- Reconstruction from keyframes and patches
- Index lookup correctness
- Config parsing

Manual validation:

- Run the recorder for a work session
- Confirm idle periods generate minimal output
- Confirm typical activities produce reconstructable history
- Confirm viewer timeline navigation remains responsive

## Implementation Phasing

### Phase 1

- Create Rust workspace and baseline configuration
- Implement session creation and manifest writing
- Implement mock or basic frame source abstraction
- Implement diff pipeline on in-memory frames
- Implement patch and keyframe persistence
- Implement reconstruction logic

### Phase 2

- Integrate real Windows screen capture
- Add background recorder loop
- Add local viewer with timeline scrub

### Phase 3

- Tune thresholds and storage behavior with profiling
- Add summary metadata for faster browsing
- Improve resilience and operational polish

## Open Questions

These are intentionally deferred until implementation or real-world tuning:

- Exact capture backend crate choice for Windows Graphics Capture
- Exact block encoding format for patches
- Whether thumbnails belong in the main session format or are generated lazily
- Whether the viewer should be embedded or launched as a local web page

## Success Criteria

The design is successful when:

- The user can start the recorder once and leave it running all day.
- Idle periods generate little or no additional disk output.
- Small localized changes do not force full-frame storage.
- The user can later scrub a timeline and inspect reconstructed screen states.
- Default settings prioritize low resource usage over visual fidelity.
