# Screen Timeline Recorder

Windows-first screen recorder for long-running, time-indexed desktop history with a built-in local viewer, tray controls, autostart, and ffmpeg-based video segments.

## What It Does

- Records the primary display on Windows with low-overhead polling.
- Stores each recording as a session under `output/sessions/<session_id>`.
- Ships a local viewer for browsing recent sessions, playback, deletion, and recorder controls.
- Supports tray-based desktop mode, global shortcuts, and login autostart.
- Supports video-segment recording with optional wall-clock burn-in.
- Includes retention controls for max session count, max age, and max total bytes.

## Release Package

Current release packaging is a portable Windows bundle:

- `screen-timeline-recorder.exe`
- `viewer/`
- `icons/`
- `ffmpeg/ffmpeg.exe`
- `README.txt`

Use the release zip if you want the fastest path for a new machine. No separate installer is required for the portable bundle.

## Quick Start

### Option 1: Use The Release Zip

1. Download the latest release zip from GitHub Releases.
2. Extract it to any writable folder, for example `D:\Apps\screen-timeline-recorder`.
3. Run:

```powershell
.\screen-timeline-recorder.exe desktop --output-dir .\output
```

To start hidden in the tray:

```powershell
.\screen-timeline-recorder.exe desktop --background --output-dir .\output
```

To start hidden and begin recording immediately:

```powershell
.\screen-timeline-recorder.exe desktop --background --autorun-record --output-dir .\output
```

### Option 2: Build From Source

```powershell
cargo build
```

Start recording:

```powershell
cargo run -- record-video --output-dir .\output
```

Open the latest recorded session in the local viewer:

```powershell
cargo run -- view-latest --output-dir .\output
```

Open desktop mode:

```powershell
cargo run -- desktop --output-dir .\output
```

## Desktop Mode

`desktop` launches the embedded viewer window, keeps the tray icon alive, and registers these global shortcuts:

- `Ctrl+Alt+Shift+R`: start a new recording
- `Ctrl+Alt+Shift+P`: pause or resume the active recording
- `Ctrl+Alt+Shift+S`: stop the active recording
- `Ctrl+Alt+Shift+O`: show the main window

The tray menu can also open the window, start recording, pause, resume, stop, and quit.

## Viewer

The local viewer is served from `viewer/` and exposes JSON endpoints under `/api`.

- Recent sessions list with duration and size
- Native video playback for segmented recordings
- Start, stop, refresh, delete, and status controls
- Chinese and English UI
- Autostart and recording settings in-page

Default local bind address is `127.0.0.1:8080`.

## Autostart

The UI can configure Windows login autostart through a Scheduled Task. The task launches desktop mode in background mode and can optionally start recording immediately after login.

## Recording Format

### Video-Segment Sessions

- ffmpeg sidecar recording
- H.264 MP4 segments
- Optional wall-clock burn-in
- Session metadata and segment index for playback

The bundled video recorder expects `ffmpeg\ffmpeg.exe` beside the app, or `SCREEN_TIMELINE_FFMPEG` to point at a valid ffmpeg binary.

## Retention

The recorder can prune old recordings before starting a new session:

- `max_sessions`
- `max_age_days`
- `max_total_bytes`

If none are configured, recordings accumulate under `output/sessions`.

## CLI

Common commands:

```powershell
cargo run -- record-video --output-dir .\output
cargo run -- view <session_id> --output-dir .\output
cargo run -- view-latest --output-dir .\output
cargo run -- desktop --output-dir .\output
cargo run -- pause <session_id> --output-dir .\output
cargo run -- resume <session_id> --output-dir .\output
cargo run -- stop <session_id> --output-dir .\output
cargo run -- status <session_id> --output-dir .\output
```

## Validation

Run tests:

```powershell
cargo test
```

Quick Windows smoke:

```powershell
powershell -ExecutionPolicy Bypass -File .\scripts\soak.ps1 -RecordSeconds 15 -PauseResume
```

Package the portable desktop bundle:

```powershell
powershell -ExecutionPolicy Bypass -File .\scripts\package-desktop.ps1
```

## Limitations

- Windows-only for actual capture.
- Current capture path is functional but not yet optimized for very long retention-heavy installs.
- Release packaging is currently a portable zip bundle instead of a dedicated Windows installer.

## License

MIT
