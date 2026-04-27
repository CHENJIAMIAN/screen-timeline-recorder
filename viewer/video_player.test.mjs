import test from "node:test";
import assert from "node:assert/strict";

import {
  advanceHighSpeedPosition,
  applyPlaybackPreferences,
  armPlaybackPreferenceSync,
  isHighSpeedPlayback,
  nativePlaybackRateForSpeed,
  playbackSpeedOptions,
  segmentIndexFromSliderValue,
  sliderValueFromSegmentIndex,
} from "./video_player.js";

class FakeVideoPlayer {
  constructor() {
    this.playbackRate = 1;
    this.defaultPlaybackRate = 1;
    this.loop = true;
    this.src = "";
    this.listeners = new Map();
    this.playCalls = 0;
  }

  addEventListener(type, handler) {
    if (!this.listeners.has(type)) {
      this.listeners.set(type, new Set());
    }
    this.listeners.get(type).add(handler);
  }

  removeEventListener(type, handler) {
    this.listeners.get(type)?.delete(handler);
  }

  dispatch(type) {
    for (const handler of this.listeners.get(type) || []) {
      handler();
    }
  }

  load() {
    this.playbackRate = 1;
    this.defaultPlaybackRate = 1;
  }

  play() {
    this.playCalls += 1;
    return Promise.resolve();
  }
}

test("applyPlaybackPreferences writes both playbackRate and defaultPlaybackRate", () => {
  const player = new FakeVideoPlayer();

  applyPlaybackPreferences(player, "32");

  assert.equal(player.playbackRate, 16);
  assert.equal(player.defaultPlaybackRate, 16);
  assert.equal(player.loop, false);
});

test("nativePlaybackRateForSpeed caps browser playback while detecting high speed mode", () => {
  assert.equal(nativePlaybackRateForSpeed("8"), 8);
  assert.equal(nativePlaybackRateForSpeed("16"), 16);
  assert.equal(nativePlaybackRateForSpeed("32"), 16);
  assert.equal(nativePlaybackRateForSpeed("360"), 16);
  assert.equal(isHighSpeedPlayback("16"), false);
  assert.equal(isHighSpeedPlayback("32"), true);
});

test("armPlaybackPreferenceSync reapplies the latest speed after a source load reset", () => {
  const player = new FakeVideoPlayer();
  let speed = "16";

  armPlaybackPreferenceSync(player, () => speed);
  player.load();
  assert.equal(player.playbackRate, 1);
  assert.equal(player.defaultPlaybackRate, 1);

  player.dispatch("loadedmetadata");
  assert.equal(player.playbackRate, 16);
  assert.equal(player.defaultPlaybackRate, 16);

  speed = "60";
  player.load();
  player.dispatch("canplay");
  assert.equal(player.playbackRate, 16);
  assert.equal(player.defaultPlaybackRate, 16);
});

test("armPlaybackPreferenceSync replaces old listeners so the current player state wins on next clip", () => {
  const player = new FakeVideoPlayer();

  const cleanupFirst = armPlaybackPreferenceSync(player, () => "4");
  const cleanupSecond = armPlaybackPreferenceSync(player, () => "32");

  player.load();
  player.dispatch("play");

  assert.equal(player.playbackRate, 16);
  assert.equal(player.defaultPlaybackRate, 16);

  cleanupFirst();
  cleanupSecond();
  assert.equal(player.listeners.get("loadedmetadata")?.size || 0, 0);
  assert.equal(player.listeners.get("canplay")?.size || 0, 0);
  assert.equal(player.listeners.get("play")?.size || 0, 0);
});

test("advanceHighSpeedPosition moves across 30 second segments at the selected speed", () => {
  const segments = [
    { started_at: 1_000, finished_at: 31_000 },
    { started_at: 31_000, finished_at: 61_000 },
    { started_at: 61_000, finished_at: 91_000 },
  ];

  const sameSegment = advanceHighSpeedPosition({
    segments,
    currentTimelineMs: 1_000,
    playbackSpeed: "60",
    elapsedMs: 250,
    sessionEndMs: 91_000,
    loop: false,
  });

  assert.deepEqual(sameSegment, {
    ended: false,
    segmentIndex: 0,
    offsetSeconds: 15,
    timelineMs: 16_000,
  });

  const nextSegment = advanceHighSpeedPosition({
    segments,
    currentTimelineMs: 16_000,
    playbackSpeed: "60",
    elapsedMs: 500,
    sessionEndMs: 91_000,
    loop: false,
  });

  assert.deepEqual(nextSegment, {
    ended: false,
    segmentIndex: 1,
    offsetSeconds: 15,
    timelineMs: 46_000,
  });
});

test("advanceHighSpeedPosition clamps to the final segment when high speed reaches the end", () => {
  const segments = [
    { started_at: 1_000, finished_at: 31_000 },
    { started_at: 31_000, finished_at: 61_000 },
  ];

  assert.deepEqual(
    advanceHighSpeedPosition({
      segments,
      currentTimelineMs: 51_000,
      playbackSpeed: "120",
      elapsedMs: 500,
      sessionEndMs: 61_000,
      loop: false,
    }),
    {
      ended: true,
      segmentIndex: 1,
      offsetSeconds: 30,
      timelineMs: 61_000,
    }
  );
});

test("sliderValueFromSegmentIndex converts the active segment index to a 1-based slider position", () => {
  assert.equal(sliderValueFromSegmentIndex(0), 1);
  assert.equal(sliderValueFromSegmentIndex(2), 3);
  assert.equal(sliderValueFromSegmentIndex(-1), 1);
});

test("segmentIndexFromSliderValue rounds and clamps into the available segment range", () => {
  assert.equal(segmentIndexFromSliderValue(1, 4), 0);
  assert.equal(segmentIndexFromSliderValue(2.6, 4), 2);
  assert.equal(segmentIndexFromSliderValue(99, 4), 3);
  assert.equal(segmentIndexFromSliderValue(-3, 4), 0);
  assert.equal(segmentIndexFromSliderValue(1, 0), -1);
});

test("playbackSpeedOptions includes the new 3600x option", () => {
  assert.equal(playbackSpeedOptions.at(-1), "3600");
  assert.ok(playbackSpeedOptions.includes("360"));
  assert.ok(playbackSpeedOptions.includes("3600"));
});
