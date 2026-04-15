# Narrow-screen viewer tweaks Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Keep the viewer UI usable on viewports narrower than 520px by tightening the sticky footer, inputs, and canvas without affecting desktop layouts.

**Architecture:** Single stylesheet adjustments scoped to a new `@media (max-width: 520px)` block that shrinks padding, stacks controls, and allows inputs to flex to full width while leaving desktop breakpoints untouched.

**Tech Stack:** Pure CSS residing in `viewer/styles.css`; manual validation via the existing `cargo run -- view-latest --output-dir ./output` workflow.

---

### Task 1: Add the narrow-screen media query
**Files:**
- Modify: `viewer/styles.css` (around the existing `@media (max-width: 640px)` block to keep the new rules nearby).
**Test:** Manual viewport checks by running `cargo run -- view-latest --output-dir ./output`, opening `http://127.0.0.1:8080/viewer/`, and resizing the browser to 520px, 480px, and 360px.

- [ ] **Step 1: Reproduce the current overflow by opening the viewer (via `cargo run -- view-latest --output-dir ./output`) and shrinking the window to ~520px; confirm the dock or inputs still force horizontal scroll.**
- [ ] **Step 2: Insert a new `@media (max-width: 520px)` block that forces `.viewer-dock` to use a single-column grid, reduces its padding/gap, and centers it with `width: calc(100% - 16px)` and `margin: 0 auto`.**
- [ ] **Step 3: Within the same media query, stack `.viewer-dock-actions`, set `.viewer-dock-timestamp`, `.viewer-dock-timeline`, `.field`, and `.buttons` to `width: 100%`/`min-width: 0`, and reduce `.viewer-dock` gap to `10px`.**
- [ ] **Step 4: Reload the viewer after saving and recheck the 520px/480px/360px widths; expect no horizontal scroll and all controls readable.**
- [ ] **Step 5: Run `git status` to confirm `viewer/styles.css` is staged correctly (manual commit step deferred until after plan execution).**

### Task 2: Tighten canvas, toolbar, and controls within the breakpoint
**Files:**
- Modify: `viewer/styles.css` (same media query block, adding `.viewer`, `canvas`, `.toolbar`, and `.controls` tweaks).
**Test:** Same manual viewport checks outlined above.

- [ ] **Step 1: Add within the media query `padding: 8px` and `border-radius: 8px` to `.viewer`, and ensure the `.toolbar`/`.controls` gaps shrink to `8px` so buttons wrap neatly.**
- [ ] **Step 2: Update `canvas` under the media query to `min-height: 200px`, `height: auto`, and `max-width: 100%` so it scales with the viewport.**
- [ ] **Step 3: Save and reload the viewer; verify the canvas no longer overflows and the toolbar wraps cleanly at 360px.**
- [ ] **Step 4: Confirm the sticky footer still behaves correctly by scrolling through the page; the `.viewer-dock` should remain visible and collapsed.**
- [ ] **Step 5: Run `git status` to ensure only the expected CSS file changed (capture command output if needed for later documentation).**

## Review note
Spec doc: `docs/superpowers/specs/2026-04-14-narrow-viewer-responsive-design.md`

Plan review loop: I cannot dispatch a spec- or plan-document-reviewer outside this session, so please treat the docs above as the reference. Once a reviewer is available, dispatch them with the spec and this plan, iterate until approved (max three loops), then choose either subagent-driven or inline execution per the plan completion instructions.
