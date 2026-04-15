# Narrow-screen viewer responsive design

## Context
- The viewer styles already handle desktop and two breakpoints (`1100px` and `640px`), but the sticky footer and input rows can still overflow at very narrow widths (<=520px).
- We can only modify `viewer/styles.css`, so any changes must be purely CSS-driven and scoped to narrow screens to avoid impacting desktop.

## Goals
1. Ensure the sticky `.viewer-dock` can collapse into a single, narrow column without horizontal overflow.
2. Allow all input groups, buttons, and select elements to shrink to the available width while keeping their desktop behavior untouched.
3. Reduce visual tension on the canvas and toolbar for small viewports by tightening gaps/padding and lowering the canvas min-height.

## Design

### Breakpoint gating
- Introduce a new `@media (max-width: 520px)` block that further adjusts the `.viewer-dock` layout: force a single-column grid, reduce padding/gap to 10px, and make the sticky footer width use `calc(100% - 16px)` with auto margins so it stays within the viewport.
- Inside the same media query, stack `.viewer-dock-actions` vertically, align them to stretch, and ensure the `.viewer-dock-timestamp`, `.viewer-dock-timeline`, and `.buttons` all span `100%`.
- Keep the existing larger breakpoints untouched, so desktops continue using their current padding, radius, and grid columns.

### Field and control resizing
- Within the new media query add `width: 100%`, `min-width: 0`, and `max-width: none` to elements that still have enforced minimum widths: `.field`, `.timeline-field`, `.field input`, `.timeline-field input`, `.speed-selector select`, `.language-field select`, and `.buttons`.
- Reuse the existing `.viewer-dock-actions` flex layout but set `gap: 8px` and `flex-direction: column` only within the narrow-breakpoint block.
- Leave desktop font sizes intact; only reduce gaps and margins within the media query so text legibility remains consistent outside the narrow state.

### Canvas and toolbar adjustments
- Under the same `520px` media query lower the `.viewer` padding to `8px` and reduce the `border-radius` to keep the panel from feeling overly large.
- Add `canvas { min-height: 200px; height: auto; }` to allow the drawn area to scale naturally, and ensure `canvas` keeps `max-width: 100%`.
- Tidy `.toolbar`, `.controls`, and `.session-meta` by setting `gap: 8px` and allowing their child elements to wrap cleanly within the media query, which keeps buttons accessible without shrinking sizes.

## Validation
- Manual testing must cover 360px, 480px, and 640px viewports to confirm the sticky viewer dock no longer overflows and that inputs remain usable.
- Regression risk is minimal because all adjustments are inside the new `@media` block and do not touch scripts or desktop breakpoints.

## Next steps
1. Add the new `@media (max-width: 520px)` rules as outlined above.
2. Run a quick manual check in the browser to ensure no overflow, focusing on toolbar, dock, and canvas.
3. Once the spec is reviewed, move on to the patch and validation.

Spec reviewer: please run the spec-document-reviewer subagent when able and report any issues (there is currently no automated reviewer available to me).
