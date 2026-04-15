# Screen Timeline Recorder

- Windows-first recorder for low-cost, time-indexed screen history.

## 第一次使用
1. 在项目根目录打开终端并运行 `cargo build`（或 `cargo test`）来生成可执行文件。
2. 运行 `cargo run -- record --output-dir ./output` 开始采集，按 Ctrl+C 停止；录制会自动输出内容到 `output/sessions/` 并在终端打印可供后续查看的那条会话。
3. 运行 `cargo run -- view-latest --output-dir ./output`，然后在浏览器打开默认地址（例如 `http://127.0.0.1:8080`），界面会直接打开最近的一段记录。

- Configuration is driven through TOML (`--config`) with a CLI `--output-dir` override. Key knobs include sampling interval, block size, keyframe interval, sensitivity mode, working scale, viewer defaults, and `viewer_language` (auto/en/zh) to control the viewer defaults for every session.
- When `max_sessions` is set, the recorder trims the oldest completed directories under `<output-dir>/sessions` before capturing so only the newest `N` finished sessions remain locally.
- When `max_age_days` is set, a parallel cleanup pass also removes directories older than that age; configuring both `max_sessions` and `max_age_days` applies both constraints.
- Setting `max_total_bytes` drives yet another scan: the retention pass recursively totals the manifest, keyframes, patches, and index files of each session directory and deletes the oldest sessions until the remaining directories fit under the configured byte budget.
- Frame diffing: block-based diff with a cheap sampled precheck and stability window, producing patch regions while avoiding heavier diff work during mostly static periods.
- Storage: session layout under `output/sessions/<session_id>` with `manifest.json`, keyframes, patches, and index files.
- Reconstruction: rebuilds a frame at a timestamp by applying patches onto the nearest keyframe.
- Viewer API/server: `view` serves the static viewer plus JSON/PNG endpoints for session metadata, frames, patch metadata, status heartbeat, activity, and session discovery.
- Windows capture: primary-display capture is implemented via GDI `BitBlt`.

How to run:

```bash
cargo test
```

If you want to run the helper scripts or invoke the built binary directly, build the debug executable once first:

```bash
cargo build
```

```bash
cargo run -- record --output-dir <path-to-output>
```

```bash
cargo run -- --output-dir <path-to-output>
```

Both forms start recording. `record` writes a session under `<output-dir>/sessions/<generated-session-id>` and keeps capturing until you stop it with `Ctrl+C`.
When the session stops, the CLI prints a compact summary line with frame count, duration, skipped-frame counts, diff runs, patch writes, and keyframe writes.
If `max_sessions` or `max_age_days` is configured, retention cleanup runs before a new recording starts.

```bash
cargo run -- view <session_id> --output-dir <path-to-output>
```

`view` expects existing session data at `<output-dir>/sessions/<session_id>`. Use `--bind <addr:port>` to change the default `127.0.0.1:8080`.

```bash
cargo run -- view-latest --output-dir <path-to-output>
```

`view-latest` skips the session-id lookup step and opens the most recent capture it can find under `<output-dir>/sessions`.

```bash
cargo run -- desktop --output-dir <path-to-output>
```

`desktop` launches the new Windows desktop shell that hosts the existing viewer in an embedded window, keeps a real system tray icon alive, and registers four global shortcuts:

- `Ctrl+Alt+Shift+R`: start a new recording when no recording is active
- `Ctrl+Alt+Shift+P`: pause or resume the active recording
- `Ctrl+Alt+Shift+S`: stop the active recording
- `Ctrl+Alt+Shift+O`: show the main window

```bash
cargo run -- desktop --background --autorun-record --output-dir <path-to-output>
```

This hidden-launch form is what the autostart task now targets: it starts the desktop shell silently, keeps the tray resident, and immediately begins recording in the configured output directory.

```bash
cargo run -- pause <session_id> --output-dir <path-to-output>
cargo run -- resume <session_id> --output-dir <path-to-output>
cargo run -- stop <session_id> --output-dir <path-to-output>
cargo run -- status <session_id> --output-dir <path-to-output>
```

These helper commands manipulate the same signal files and heartbeat the recorder already uses: `pause` writes `<output-dir>/sessions/<session_id>/pause.signal`, `resume` removes it, `stop` writes `<output-dir>/sessions/<session_id>/stop.signal`, and `status` pretty-prints the `status.json` heartbeat. Run the commands instead of touching files by hand while the recorder is running; you can still script the equivalent by creating or deleting the signals if you need to.

If you do not want to stop from the terminal, create `<output-dir>/sessions/<session_id>/stop.signal`; the recorder polls for that file between captured frames and exits cleanly when it appears.
If you need to temporarily hold capture without ending the session, create `<output-dir>/sessions/<session_id>/pause.signal`; while that file exists the recorder stays alive, reports `state: "paused"` in the heartbeat, and resumes capture when you remove the file.
The pause/resume flow is file-based: touching `<output-dir>/sessions/<session_id>/pause.signal` pauses after the current loop iteration and keeps polling until the file is removed, at which point capture resumes.

## Status Heartbeat

Each session directory also maintains a `status.json` heartbeat file so other local tooling can inspect whether recording is still active and read the latest cumulative stats. The file mirrors the `SessionStatus` model: `session_id`, a `state` field (`running`, `paused`, or `stopped`), the compatibility `recording` boolean, and `stats` with the running counters (`frames_seen`, `identical_frames_skipped`, `sampled_precheck_skipped`, `diff_runs`, `patch_frames_written`, `patch_regions_written`, `keyframes_written`) plus the `started_at` and `finished_at` timestamps.

The viewer server exposes `/api/status` that returns the same JSON as `status.json`, so any automation or live dashboard can poll for the latest heartbeat without reading the file directly. Polling that endpoint is also the easiest way to know whether the recorder is still running, because you can watch for the `recording` flag to flip to `false` once capture stops.

## Live Viewer

The `view` command serves the static UI from `viewer/` and exposes JSON/PNG endpoints under `/api`. On startup the viewer calls `/api/session` to load the manifest, size the canvas to `working_width`/`working_height`, and seed the timestamp controls. Use the friendly timestamp display, timeline slider, `load`/`prev`/`next` buttons, overlay toggle, and play/pause controls to inspect frames and highlight diffs.

Frames are rendered by requesting `/api/frame?ts=<timestamp>` and the overlay draws rectangles for each patch that `/api/patches?ts=<timestamp>` returns. The status line (alongside the live badge) spells out what the viewer is doing (loading a session, fetching a frame, etc.), and the timeline slider is clamped to the session start/end timestamps so you can scrub the known capture window instead of typing raw values. The UI also surfaces an activity strip and the recent-session discovery grid powered by `/api/sessions`, and the playback controls (Play/Pause toggle plus a speed selector) sit beside the slider to drive automatic stepping through the recorded timeline using the slider value as the source timestamp. The dropdown exposes the 0.25x, 0.5x, 1x, and 2x speed presets so the UI can fetch frames more or less aggressively; if those controls are still landing when you read this, rely on the timeline slider, the `Load`/`±10s` buttons, or manual timestamp entries until the playback logic is wired up.

Timestamp controls default to a friendlier label that combines `elapsed since start` with the wall-clock time, while the `Advanced Time Input` toggle reveals the hidden numeric field for typing raw millisecond values. Every control (buttons, slider, API requests, and query parameters) still operates on the same millisecond timestamps so you can capture and replay with precise timing across tools.

Recent Sessions cards now also list each session's approximate on-disk size (the server totals the manifest, keyframes, patches, and index files recursively) so you can see which captures are consuming space before retention cleanup runs.

The viewer supports Chinese and English. `viewer_language` in config controls the default (`auto`, `en`, or `zh`), and each session records the current preference in its manifest. When `viewer_language` is `auto`, the viewer prefers `zh` whenever the browser language starts with `zh` and falls back to `en` otherwise; you can override the language selector or append `?lang=en`/`?lang=zh` to lock the UI regardless of the recording or browser defaults (the choice is also saved in localStorage).

Note: loop playback simply replays the recorded patches in sequence so you can inspect behavior repeatedly without creating another capture, and burning timestamps into frames is meant as a reliability/proof indicator—expect the extra pixels to increase storage churn since every frame must carry the overlay.

## Validation Helper

For a quick Windows smoke or short soak run, use:

```powershell
powershell -ExecutionPolicy Bypass -File .\scripts\soak.ps1 -RecordSeconds 15 -PauseResume
```

The helper expects `target/debug/screen-timeline-recorder.exe` to already exist, uses the repo-local `output/sessions` layout by default, and when `-PauseResume` is passed it automatically pauses and resumes the active session through the same signal files the recorder watches. It starts recording, discovers the new session, optionally pauses and resumes it, stops it, reads `status.json`, and prints a compact summary including final state and on-disk byte size.

## Limitations

- The current Windows capture path uses a simple GDI polling loop. It is functional, but not yet optimized for very long-running low-overhead recording.
- Retention only runs when you configure `max_sessions`, `max_age_days`, or `max_total_bytes`; otherwise the recorder keeps accumulating data under `output/sessions`.
- Non-Windows platforms still report recording as unsupported.
