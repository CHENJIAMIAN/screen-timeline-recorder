const test = require("node:test");
const assert = require("node:assert/strict");

const {
  resolveControlTarget,
  computeControlButtons,
} = require("./control_logic.js");

test("targets the selected session when it is actively recording", () => {
  const target = resolveControlTarget({
    currentSessionId: "session-running",
    currentLiveState: "running",
    sessions: [
      { session_id: "session-running", status: { state: "running", recording: true } },
      { session_id: "session-stopped", status: { state: "stopped", recording: false } },
    ],
  });

  assert.deepEqual(target, {
    sessionId: "session-running",
    liveState: "running",
    source: "current",
  });
});

test("targets another active session when the selected session is stopped", () => {
  const target = resolveControlTarget({
    currentSessionId: "session-stopped",
    currentLiveState: "stopped",
    sessions: [
      { session_id: "session-running", status: { state: "running", recording: true } },
      { session_id: "session-stopped", status: { state: "stopped", recording: false } },
    ],
  });

  assert.deepEqual(target, {
    sessionId: "session-running",
    liveState: "running",
    source: "active-session",
  });
});

test("enables stop for the active session even when another stopped session is selected", () => {
  const buttons = computeControlButtons({
    currentSessionId: "session-stopped",
    currentLiveState: "stopped",
    sessions: [
      { session_id: "session-running", status: { state: "running", recording: true } },
      { session_id: "session-stopped", status: { state: "stopped", recording: false } },
    ],
  });

  assert.equal(buttons.startDisabled, true);
  assert.equal(buttons.pauseDisabled, false);
  assert.equal(buttons.resumeDisabled, true);
  assert.equal(buttons.stopDisabled, false);
  assert.equal(buttons.controlSessionId, "session-running");
  assert.equal(buttons.controlLiveState, "running");
});

test("enables resume and stop when the active session is paused", () => {
  const buttons = computeControlButtons({
    currentSessionId: "session-stopped",
    currentLiveState: "stopped",
    sessions: [
      { session_id: "session-paused", status: { state: "paused", recording: true } },
      { session_id: "session-stopped", status: { state: "stopped", recording: false } },
    ],
  });

  assert.equal(buttons.startDisabled, true);
  assert.equal(buttons.pauseDisabled, true);
  assert.equal(buttons.resumeDisabled, false);
  assert.equal(buttons.stopDisabled, false);
  assert.equal(buttons.controlSessionId, "session-paused");
  assert.equal(buttons.controlLiveState, "paused");
});
