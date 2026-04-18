# Video Segment Recorder Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Replace the current patch-based screen recorder core with segmented standard-video recording while keeping the existing Windows desktop shell, tray, startup, and session-management product surface.

**Architecture:** Keep the current `desktop` command, control endpoints, and session directories, but change the session payload from keyframes/patches to time-indexed video segments plus lightweight metadata. Recording becomes an `ffmpeg` sidecar process managed by the desktop app/control APIs, and playback switches from reconstructed PNG frames to HTML5 video playback over segment manifests.

**Tech Stack:** Rust, Tauri desktop shell, `tiny_http` viewer server, HTML/CSS/JS viewer, Windows `ffmpeg` sidecar, JSON manifests.

---

### Task 1: Introduce video-session data model without breaking existing desktop shell

**Files:**
- Create: `src/video_session.rs`
- Modify: `src/lib.rs`
- Modify: `src/session.rs`
- Test: `tests/video_session_tests.rs`

- [ ] **Step 1: Write the failing test**

Create tests for:
- creating a video session manifest with `format = "video-segments"`
- writing and reloading a segment index entry
- preserving existing user-facing session identifiers and start/end timestamps

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test --test video_session_tests`
Expected: FAIL because `video_session` module does not exist.

- [ ] **Step 3: Write minimal implementation**

Implement:
- a `VideoSessionManifest`
- a `VideoSegmentEntry`
- helpers to save/load segment index JSONL or JSON
- backward-compatible session metadata fields used by session list and viewer

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test --test video_session_tests`
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add src/video_session.rs src/lib.rs src/session.rs tests/video_session_tests.rs
git commit -m "feat: add video session metadata model"
```

### Task 2: Add ffmpeg sidecar discovery and recorder process management

**Files:**
- Create: `src/video_recorder.rs`
- Modify: `src/main.rs`
- Modify: `src/cli.rs`
- Modify: `src/desktop.rs`
- Test: `tests/video_recorder_tests.rs`

- [ ] **Step 1: Write the failing test**

Create tests for:
- discovering an `ffmpeg` binary from bundled path or configured path
- generating correct segmented recording command arguments
- creating session status for `running`, `paused`, and `stopped`

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test --test video_recorder_tests`
Expected: FAIL because sidecar discovery and command builder are missing.

- [ ] **Step 3: Write minimal implementation**

Implement:
- ffmpeg path resolution
- command builder for Windows desktop capture to segmented MP4
- recorder session bootstrap that creates session folders and launches ffmpeg
- control helpers that stop/rotate the recorder cleanly

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test --test video_recorder_tests`
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add src/video_recorder.rs src/main.rs src/cli.rs src/desktop.rs tests/video_recorder_tests.rs
git commit -m "feat: launch ffmpeg video recorder sidecar"
```

### Task 3: Expose video-session APIs to the existing viewer server

**Files:**
- Modify: `src/viewer_api.rs`
- Modify: `src/viewer_server.rs`
- Modify: `src/session_control.rs`
- Test: `tests/viewer_api_video_tests.rs`
- Test: `tests/viewer_server_tests.rs`

- [ ] **Step 1: Write the failing test**

Add tests for:
- `/api/session` returning `recording_format = "video-segments"`
- `/api/segments` returning segment URLs and start/end timestamps
- existing start/stop/delete flows working against video sessions

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test --test viewer_api_video_tests`
Expected: FAIL because segment APIs are missing.

- [ ] **Step 3: Write minimal implementation**

Implement:
- session list/status integration for video sessions
- segment-list endpoint
- static serving of recorded segment files
- compatibility handling for old patch sessions if they still exist on disk

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test --test viewer_api_video_tests`
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add src/viewer_api.rs src/viewer_server.rs src/session_control.rs tests/viewer_api_video_tests.rs tests/viewer_server_tests.rs
git commit -m "feat: add video segment viewer APIs"
```

### Task 4: Replace canvas replay with HTML5 video segment playback

**Files:**
- Modify: `viewer/index.html`
- Modify: `viewer/app.js`
- Modify: `viewer/styles.css`
- Test: `tests/viewer_server_tests.rs`

- [ ] **Step 1: Write the failing test**

Add tests for:
- viewer HTML including a video element and segment-driven controls
- removal of patch-overlay-dependent playback requirement for video sessions

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test --test viewer_server_tests serves_index_html_for_root`
Expected: FAIL because the viewer still renders canvas-only replay.

- [ ] **Step 3: Write minimal implementation**

Implement:
- `<video>` playback surface
- segment preloading and seek mapping
- loop, speed, and timeline synchronization against segment boundaries
- legacy-session fallback if a patch session is opened

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test --test viewer_server_tests`
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add viewer/index.html viewer/app.js viewer/styles.css tests/viewer_server_tests.rs
git commit -m "feat: switch viewer playback to video segments"
```

### Task 5: Package and validate bundled ffmpeg sidecar flow

**Files:**
- Create: `scripts/fetch-ffmpeg.ps1`
- Modify: `scripts/package-desktop.ps1`
- Modify: `src/autostart.rs`
- Test: `tests/autostart_tests.rs`

- [ ] **Step 1: Write the failing test**

Add tests for:
- packaged desktop output including the ffmpeg sidecar directory
- autostart command resolving bundled ffmpeg-enabled desktop mode without breaking existing startup

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test --test autostart_tests`
Expected: FAIL because packaging and startup do not account for the sidecar.

- [ ] **Step 3: Write minimal implementation**

Implement:
- deterministic bundled ffmpeg location
- packaging script update
- autostart/start-recording path validation

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test --test autostart_tests`
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add scripts/fetch-ffmpeg.ps1 scripts/package-desktop.ps1 src/autostart.rs tests/autostart_tests.rs
git commit -m "feat: package ffmpeg sidecar for desktop distribution"
```

### Task 6: Final verification and release smoke test

**Files:**
- Modify: `README.md`
- Modify: `docs/`

- [ ] **Step 1: Run targeted tests**

Run:
- `cargo test --test video_session_tests`
- `cargo test --test video_recorder_tests`
- `cargo test --test viewer_api_video_tests`
- `cargo test --test viewer_server_tests`

- [ ] **Step 2: Run integration build**

Run: `cargo build --release`
Expected: PASS

- [ ] **Step 3: Launch the desktop app**

Run:

```powershell
Start-Process -FilePath 'D:\Desktop\screen-timeline-recorder\target\release\screen-timeline-recorder.exe' -ArgumentList @('desktop','--output-dir','D:\Desktop\screen-timeline-recorder\output')
```

Expected: desktop UI opens or background tray process stays alive and can start recording.

- [ ] **Step 4: Update docs**

Document:
- video segment architecture
- ffmpeg sidecar requirement
- legacy patch-session compatibility note

- [ ] **Step 5: Commit**

```bash
git add README.md docs
git commit -m "docs: describe video segment recording architecture"
```
