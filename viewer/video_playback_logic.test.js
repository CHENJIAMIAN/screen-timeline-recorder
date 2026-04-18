const test = require("node:test");
const assert = require("node:assert/strict");

const {
  findVideoSegmentIndex,
  getVideoSegmentEndMs,
  getVideoTargetTimeSeconds,
  shouldSeekVideo,
} = require("./video_playback_logic.js");

test("finds the matching segment for timestamps across segment boundaries", () => {
  const segments = [
    { started_at: 1_000, finished_at: 4_000 },
    { started_at: 4_000, finished_at: 8_000 },
  ];

  assert.equal(findVideoSegmentIndex(segments, 1_000, 8_000), 0);
  assert.equal(findVideoSegmentIndex(segments, 3_999, 8_000), 0);
  assert.equal(findVideoSegmentIndex(segments, 4_000, 8_000), 1);
  assert.equal(findVideoSegmentIndex(segments, 7_999, 8_000), 1);
  assert.equal(findVideoSegmentIndex(segments, 8_000, 8_000), 1);
});

test("uses the next segment start as a fallback end when finished_at is missing", () => {
  const segments = [
    { started_at: 1_000 },
    { started_at: 4_000 },
    { started_at: 7_500 },
  ];

  assert.equal(getVideoSegmentEndMs(segments, 0, 9_000), 4_000);
  assert.equal(getVideoSegmentEndMs(segments, 1, 9_000), 7_500);
  assert.equal(getVideoSegmentEndMs(segments, 2, 9_000), 9_000);
});

test("converts absolute timestamps into per-segment video offsets", () => {
  const segment = { started_at: 12_000 };

  assert.equal(getVideoTargetTimeSeconds(segment, 12_000), 0);
  assert.equal(getVideoTargetTimeSeconds(segment, 13_500), 1.5);
});

test("only seeks when the target diverges materially from the current video position", () => {
  assert.equal(shouldSeekVideo(12.0, 12.05), false);
  assert.equal(shouldSeekVideo(12.0, 12.35), true);
  assert.equal(shouldSeekVideo(Number.NaN, 5.0), true);
});
