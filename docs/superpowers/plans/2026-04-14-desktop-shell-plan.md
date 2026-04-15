# Desktop Shell Integration Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Turn the existing recorder/viewer into a true Windows desktop app with a single executable path toward tray, global shortcuts, and silent background startup.

**Architecture:** Keep the current Rust recording/storage/viewer core intact and add a Tauri desktop shell around it. The first phase introduces an embedded desktop window and a desktop-mode runtime entrypoint while preserving the existing CLI commands for record/view/control automation.

**Tech Stack:** Rust, Tauri v2, existing static viewer assets, existing recorder/session APIs.

---

### Task 1: Add desktop-mode runtime scaffolding

**Files:**
- Modify: `Cargo.toml`
- Modify: `src/cli.rs`
- Modify: `src/main.rs`
- Create: `src/desktop.rs`
- Test: `tests/cli_tests.rs`

- [ ] Add a `desktop` command (and/or no-arg desktop default on Windows) to the CLI model without breaking existing record/view/control commands.
- [ ] Write or extend CLI tests so desktop-mode parsing is covered first.
- [ ] Add the minimal Tauri dependencies and build plumbing required to compile a desktop shell.
- [ ] Split `main.rs` so CLI execution stays intact while desktop mode delegates into `desktop.rs`.
- [ ] Re-run CLI tests and ensure non-desktop commands still behave exactly as before.

### Task 2: Embed the existing viewer into a desktop window

**Files:**
- Modify: `src/viewer_server.rs`
- Modify: `src/lib.rs`
- Create: `src/desktop.rs`
- Create: `build.rs`
- Create: `tauri.conf.json` or equivalent Tauri config files
- Test: targeted `cargo test` suites plus a manual launch

- [ ] Expose a desktop-safe asset-loading path so the viewer HTML/CSS/JS can be embedded and served without a sibling `viewer/` directory requirement.
- [ ] Start a local viewer host from desktop mode and open it inside a Tauri window instead of an external browser.
- [ ] Ensure the desktop window can open the latest session and still work with the existing viewer UI.
- [ ] Verify the standalone CLI `view` and `view-latest` flows still work outside desktop mode.

### Task 3: Prepare the shell for tray, hotkeys, and silent startup

**Files:**
- Modify: `Cargo.toml`
- Modify: `src/desktop.rs`
- Create/Modify: Tauri capability and plugin config files
- Test: manual smoke checks

- [ ] Wire in the Tauri plugins and app state needed for system tray, global shortcut registration, and autostart.
- [ ] Add placeholder tray menu structure and command routing using the existing Rust session control functions.
- [ ] Add placeholder global shortcut registration for the four approved actions: start, pause/resume, stop, open UI.
- [ ] Keep this phase functional but thin: the goal is a stable shell foundation, not polished UX yet.

### Task 4: Verify the first desktop-shell cut

**Files:**
- Modify: `README.md`
- Test: `cargo test`, desktop manual smoke run

- [ ] Run the full Rust test suite.
- [ ] Launch desktop mode locally and confirm the window opens with the existing viewer content.
- [ ] Document the new desktop mode entrypoint and current limitations.
