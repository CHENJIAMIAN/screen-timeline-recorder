# Screen Timeline Recorder Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build a Windows-first recorder that captures low-cost screen history as keyframes plus changed-region patches and supports local timeline reconstruction.

**Architecture:** Use a single Rust crate with a reusable library surface in `src/lib.rs` plus a CLI binary in `src/main.rs`. The library owns configuration, frame models, diffing, session storage, indexing, reconstruction, and capture backends; the binary wires recording and viewing commands on top.

**Tech Stack:** Rust, Cargo, Serde, image/png encoding, Windows capture integration, local static web viewer or lightweight HTTP serving.

---

## Planned File Structure

### Rust crate

- Create: `Cargo.toml`
- Create: `src/lib.rs`
- Create: `src/main.rs`
- Create: `src/cli.rs`
- Create: `src/config.rs`
- Create: `src/session.rs`
- Create: `src/frame.rs`
- Create: `src/diff.rs`
- Create: `src/storage.rs`
- Create: `src/index.rs`
- Create: `src/reconstruct.rs`
- Create: `src/logging.rs`
- Create: `src/capture/mod.rs`
- Create: `src/capture/mock.rs`
- Create: `src/capture/windows.rs`

### Integration tests

- Create: `tests/config_tests.rs`
- Create: `tests/diff_tests.rs`
- Create: `tests/storage_tests.rs`
- Create: `tests/reconstruct_tests.rs`
- Create: `tests/recorder_loop_tests.rs`
- Create: `tests/windows_capture_tests.rs`
- Create: `tests/viewer_api_tests.rs`
- Create: `tests/resilience_tests.rs`

### Viewer assets

- Create: `viewer/index.html`
- Create: `viewer/app.js`
- Create: `viewer/styles.css`

### Project docs

- Create: `README.md`

## Canonical Session Format

The implementation must persist sessions exactly under:

```text
sessions/YYYY-MM-DD/
  manifest.json
  keyframes/
  patches/
  index/
```

Minimum manifest fields that tests must assert:

- `session_id`
- `started_at`
- `finished_at`
- `display_width`
- `display_height`
- `working_width`
- `working_height`
- `sampling_interval_ms`
- `block_width`
- `block_height`
- `keyframe_interval_ms`
- `sensitivity_mode`
- `precheck_threshold`
- `block_difference_threshold`
- `changed_pixel_ratio_threshold`
- `stability_window`
- `compression_format`
- `recorder_version`
- `viewer_default_zoom`
- `viewer_overlay_enabled_by_default`

Session root rule:

- `output_dir` is user-configurable
- canonical session layout is created under `<output_dir>/sessions/YYYY-MM-DD/`

Minimum index files for v1:

- `index/keyframes.jsonl` with timestamp-to-keyframe-path entries
- `index/patches.jsonl` with timestamp-to-patch-path entries and sequence numbers

Viewer contract for v1:

- CLI starts a local HTTP server
- `GET /api/session` returns manifest and summary metadata
- `GET /api/frame?ts=<millis>` returns a PNG for the reconstructed frame at timestamp
- `GET /api/patches?ts=<millis>` returns changed-region metadata for overlay rendering

## Task 1: Bootstrap the Rust crate and configuration model

**Files:**
- Create: `Cargo.toml`
- Create: `src/lib.rs`
- Create: `src/main.rs`
- Create: `src/cli.rs`
- Create: `src/config.rs`
- Create: `tests/config_tests.rs`
- Create: `README.md`

- [ ] **Step 1: Write the failing configuration tests**

Add `tests/config_tests.rs` with tests for:
- default configuration values
- parsing from a local config file
- honoring a user-provided `output_dir`
- rejecting invalid values such as zero block size, zero keyframe interval, or invalid working scale
- sensitivity mode mapping to explicit thresholds for pre-check, block difference, changed-pixel ratio, and stability window
- viewer default parsing for zoom and overlay visibility

- [ ] **Step 2: Run the configuration tests and verify they fail**

Run: `cargo test --test config_tests`
Expected: fail because config module and crate structure do not exist yet

- [ ] **Step 3: Create the Cargo project and minimal config implementation**

Add:
- package metadata and dependencies in `Cargo.toml`
- `src/lib.rs` exporting config and CLI-facing library modules
- CLI entry point in `src/main.rs`
- command parsing skeleton in `src/cli.rs`
- `RecorderConfig` and validation helpers in `src/config.rs`

Include only fields required by the spec and canonical session format.
Ensure the config model and CLI surface both expose `output_dir`.
Include viewer default fields for zoom and overlay visibility.

- [ ] **Step 4: Re-run the configuration tests and verify they pass**

Run: `cargo test --test config_tests`
Expected: PASS

- [ ] **Step 5: Run formatting and full test suite**

Run: `cargo fmt --check`
Run: `cargo test`
Expected: all current tests pass

- [ ] **Step 6: Commit**

```bash
git add Cargo.toml src/lib.rs src/main.rs src/cli.rs src/config.rs tests/config_tests.rs README.md
git commit -m "feat: bootstrap recorder config and cli skeleton"
```

## Task 2: Implement frame model and diff engine

**Files:**
- Create: `src/frame.rs`
- Create: `src/diff.rs`
- Create: `tests/diff_tests.rs`
- Modify: `src/lib.rs`
- Modify: `src/config.rs`

- [ ] **Step 1: Write the failing diff tests**

Add `tests/diff_tests.rs` covering:
- unchanged frames produce no patches
- low-resolution pre-check skips block diff for nearly identical frames
- a single changed block yields one patch
- tiny sub-threshold changes are ignored
- changed-pixel ratio threshold is enforced
- transient one-sample noise is ignored when stability filtering is enabled
- stable repeated changes are emitted after the stability window is satisfied

- [ ] **Step 2: Run the diff tests and verify they fail**

Run: `cargo test --test diff_tests`
Expected: fail because frame and diff modules do not exist yet

- [ ] **Step 3: Implement minimal frame and diff code**

Add:
- `Frame` struct with width, height, and RGBA buffer
- helper constructors for tests
- low-resolution pre-check implementation
- block diff logic
- threshold configuration sourced from `RecorderConfig`
- patch metadata model and temporal stability tracking

- [ ] **Step 4: Re-run the diff tests and verify they pass**

Run: `cargo test --test diff_tests`
Expected: PASS

- [ ] **Step 5: Run formatting and full test suite**

Run: `cargo fmt --check`
Run: `cargo test`
Expected: all current tests pass

- [ ] **Step 6: Commit**

```bash
git add src/lib.rs src/config.rs src/frame.rs src/diff.rs tests/diff_tests.rs
git commit -m "feat: add block diff pipeline"
```

## Task 3: Implement session manifest, patch storage, and indexes

**Files:**
- Create: `src/session.rs`
- Create: `src/storage.rs`
- Create: `src/index.rs`
- Create: `tests/storage_tests.rs`
- Modify: `src/lib.rs`
- Modify: `src/config.rs`
- Modify: `src/diff.rs`

- [ ] **Step 1: Write the failing storage tests**

Add `tests/storage_tests.rs` covering:
- session directory creation with exact `sessions/YYYY-MM-DD/{manifest.json,keyframes,patches,index}` layout
- creation of the canonical layout beneath a user-provided `output_dir`
- manifest persistence with all required fields
- keyframe write and `index/keyframes.jsonl` entry creation
- patch persistence with timestamped metadata and `index/patches.jsonl` entry creation
- no-op write path for unchanged frames
- rapid repeated writes to the same region are coalesced into fewer persisted patch records
- index lookup uses index files rather than directory scanning

- [ ] **Step 2: Run the storage tests and verify they fail**

Run: `cargo test --test storage_tests`
Expected: fail because session, storage, and index modules do not exist yet

- [ ] **Step 3: Implement minimal storage code**

Add:
- session directory creation helpers
- manifest serialization with serde
- keyframe writer
- patch writer using a simple stable image encoding
- minimal patch coalescing for rapid repeated writes to the same region before persistence
- index writers for `keyframes.jsonl` and `patches.jsonl`
- lookup helpers that read index files directly

- [ ] **Step 4: Re-run the storage tests and verify they pass**

Run: `cargo test --test storage_tests`
Expected: PASS

- [ ] **Step 5: Run formatting and full test suite**

Run: `cargo fmt --check`
Run: `cargo test`
Expected: all current tests pass

- [ ] **Step 6: Commit**

```bash
git add src/lib.rs src/config.rs src/diff.rs src/session.rs src/storage.rs src/index.rs tests/storage_tests.rs
git commit -m "feat: add session storage primitives"
```

## Task 4: Implement timeline reconstruction

**Files:**
- Create: `src/reconstruct.rs`
- Create: `tests/reconstruct_tests.rs`
- Modify: `src/lib.rs`
- Modify: `src/frame.rs`
- Modify: `src/session.rs`
- Modify: `src/storage.rs`
- Modify: `src/index.rs`

- [ ] **Step 1: Write the failing reconstruction tests**

Add `tests/reconstruct_tests.rs` covering:
- reconstructing from a keyframe alone
- reconstructing after multiple patches
- selecting the nearest prior keyframe using `index/keyframes.jsonl`
- replaying patches in timestamp and sequence order using `index/patches.jsonl`

- [ ] **Step 2: Run the reconstruction tests and verify they fail**

Run: `cargo test --test reconstruct_tests`
Expected: fail because reconstruction module does not exist yet

- [ ] **Step 3: Implement minimal reconstruction code**

Add:
- session reader helpers
- nearest-keyframe lookup through the index module
- patch replay over a mutable frame
- timestamp-based reconstruction API

- [ ] **Step 4: Re-run the reconstruction tests and verify they pass**

Run: `cargo test --test reconstruct_tests`
Expected: PASS

- [ ] **Step 5: Run formatting and full test suite**

Run: `cargo fmt --check`
Run: `cargo test`
Expected: all current tests pass

- [ ] **Step 6: Commit**

```bash
git add src/lib.rs src/frame.rs src/session.rs src/storage.rs src/index.rs src/reconstruct.rs tests/reconstruct_tests.rs
git commit -m "feat: add timeline reconstruction"
```

## Task 5: Integrate recorder loop with mock capture source

**Files:**
- Create: `src/capture/mod.rs`
- Create: `src/capture/mock.rs`
- Create: `tests/recorder_loop_tests.rs`
- Modify: `src/lib.rs`
- Modify: `src/main.rs`
- Modify: `src/cli.rs`
- Modify: `src/config.rs`
- Modify: `src/diff.rs`
- Modify: `src/storage.rs`
- Modify: `src/session.rs`

- [ ] **Step 1: Write the failing recorder loop tests**

Add `tests/recorder_loop_tests.rs` with named tests for:
- `recorder_loop_skips_storage_when_frames_do_not_change`
- `recorder_loop_writes_patch_when_frame_changes`
- `recorder_loop_writes_periodic_keyframe_even_without_new_changes`
- `recorder_loop_handles_capture_interruptions_and_finalizes_manifest`
- `recorder_loop_finalizes_manifest_on_shutdown`

- [ ] **Step 2: Run the recorder loop tests and verify they fail**

Run: `cargo test --test recorder_loop_tests`
Expected: fail because recorder orchestration does not exist yet

- [ ] **Step 3: Implement minimal recorder orchestration**

Add:
- capture trait in `src/capture/mod.rs`
- deterministic mock capture source in `src/capture/mock.rs`
- recorder loop wiring capture, diff, storage, and periodic keyframe policy
- capture interruption handling policy that stops the loop cleanly with a logged error and finalized manifest
- graceful shutdown path that finalizes `finished_at` in the manifest
- CLI command for starting a recording session with mock mode for tests

- [ ] **Step 4: Re-run the recorder loop tests and verify they pass**

Run: `cargo test --test recorder_loop_tests`
Expected: PASS

- [ ] **Step 5: Run formatting and full test suite**

Run: `cargo fmt --check`
Run: `cargo test`
Expected: all current tests pass

- [ ] **Step 6: Commit**

```bash
git add src/lib.rs src/main.rs src/cli.rs src/config.rs src/diff.rs src/storage.rs src/session.rs src/capture/mod.rs src/capture/mock.rs tests/recorder_loop_tests.rs
git commit -m "feat: wire recorder loop with mock capture"
```

## Task 6: Add resilience and logging behavior

**Files:**
- Create: `src/logging.rs`
- Create: `tests/resilience_tests.rs`
- Modify: `src/lib.rs`
- Modify: `src/storage.rs`
- Modify: `src/session.rs`
- Modify: `src/reconstruct.rs`
- Modify: `src/cli.rs`

- [ ] **Step 1: Write the failing resilience tests**

Add `tests/resilience_tests.rs` covering:
- storage write failure surfaces a structured error
- simulated disk-full error stops recording cleanly, leaves the partial session readable, and finalizes the manifest
- corrupted patch file does not make the entire session unreadable
- graceful shutdown still finalizes the manifest after earlier recoverable errors

- [ ] **Step 2: Run the resilience tests and verify they fail**

Run: `cargo test --test resilience_tests`
Expected: fail because resilience helpers and logging module do not exist yet

- [ ] **Step 3: Implement minimal resilience behavior**

Add:
- structured logging helpers
- recoverable error types for storage and reconstruction
- manifest finalization helper used during shutdown paths
- corrupt-patch skip behavior with logged warning

- [ ] **Step 4: Re-run the resilience tests and verify they pass**

Run: `cargo test --test resilience_tests`
Expected: PASS

- [ ] **Step 5: Run formatting and full test suite**

Run: `cargo fmt --check`
Run: `cargo test`
Expected: all current tests pass

- [ ] **Step 6: Commit**

```bash
git add src/lib.rs src/storage.rs src/session.rs src/reconstruct.rs src/logging.rs src/cli.rs tests/resilience_tests.rs
git commit -m "feat: add resilience and logging"
```

## Task 7: Add Windows capture backend scaffold

**Files:**
- Create: `src/capture/windows.rs`
- Create: `tests/windows_capture_tests.rs`
- Modify: `Cargo.toml`
- Modify: `src/lib.rs`
- Modify: `src/capture/mod.rs`
- Modify: `src/cli.rs`
- Modify: `README.md`

- [ ] **Step 1: Write the failing Windows capture boundary tests**

Add `tests/windows_capture_tests.rs` covering:
- primary-display-only backend selection on supported targets
- backend selection returns the Windows backend on supported targets
- unsupported backend requests return explicit errors
- conversion from captured pixel data into internal `Frame`

- [ ] **Step 2: Run the Windows capture tests and verify they fail**

Run: `cargo test --test windows_capture_tests`
Expected: fail because Windows backend module is absent

- [ ] **Step 3: Implement minimal Windows capture scaffold**

Add:
- Windows capture backend module
- platform-gated backend selection
- primary display selection boundary for v1
- initial capture boundary and conversion glue
- explicit runtime errors for unsupported code paths

Keep the rest of the codebase platform-agnostic.

- [ ] **Step 4: Re-run the Windows capture tests and verify they pass**

Run: `cargo test --test windows_capture_tests`
Expected: PASS

- [ ] **Step 5: Run formatting and full test suite**

Run: `cargo fmt --check`
Run: `cargo test`
Expected: all current tests pass

- [ ] **Step 6: Commit**

```bash
git add Cargo.toml src/lib.rs src/capture/mod.rs src/capture/windows.rs src/cli.rs README.md tests/windows_capture_tests.rs
git commit -m "feat: add windows capture backend scaffold"
```

## Task 8: Add minimal local viewer and HTTP API

**Files:**
- Create: `viewer/index.html`
- Create: `viewer/app.js`
- Create: `viewer/styles.css`
- Create: `tests/viewer_api_tests.rs`
- Modify: `src/main.rs`
- Modify: `src/cli.rs`
- Modify: `src/reconstruct.rs`
- Modify: `src/session.rs`
- Modify: `README.md`

- [ ] **Step 1: Write the failing viewer API tests**

Add `tests/viewer_api_tests.rs` covering:
- `GET /api/session` returns manifest and summary metadata
- `GET /api/frame?ts=...` returns a PNG response
- `GET /api/patches?ts=...` returns changed-region metadata
- viewer metadata includes default zoom and overlay settings
- coarse timeline jump controls and zoom UI are wired in the served assets

- [ ] **Step 2: Run the viewer API tests and verify they fail**

Run: `cargo test --test viewer_api_tests`
Expected: fail because serving and viewer glue do not exist yet

- [ ] **Step 3: Implement the minimal viewer**

Add:
- HTTP endpoints matching the viewer contract
- basic static viewer assets
- time slider UI
- coarse jump controls for larger timeline steps
- zoom in and zoom out controls
- image rendering of reconstructed frames
- optional patch overlay toggle
- CLI command to serve the viewer

- [ ] **Step 4: Re-run the viewer API tests and verify they pass**

Run: `cargo test --test viewer_api_tests`
Expected: PASS

- [ ] **Step 5: Run formatting and full test suite**

Run: `cargo fmt --check`
Run: `cargo test`
Expected: all current tests pass

- [ ] **Step 6: Commit**

```bash
git add src/main.rs src/cli.rs src/reconstruct.rs src/session.rs viewer/index.html viewer/app.js viewer/styles.css README.md tests/viewer_api_tests.rs
git commit -m "feat: add local timeline viewer"
```

## Verification and handoff

- [ ] Run `cargo fmt --check`
- [ ] Run `cargo test`
- [ ] Run one manual end-to-end session using the mock capture source
- [ ] Verify a session directory is written with canonical layout and indexes
- [ ] Verify the viewer can scrub at least one recorded session by timestamp
- [ ] Document current limitations in `README.md`
- [ ] Prepare for final review and branch-finishing workflow
