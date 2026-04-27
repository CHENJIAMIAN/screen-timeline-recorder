const playbackSyncKey = Symbol("screenTimelinePlaybackSync");
const MAX_NATIVE_PLAYBACK_RATE = 16;

export const playbackSpeedOptions = [
  "0.25",
  "0.5",
  "0.75",
  "1",
  "1.25",
  "1.5",
  "2",
  "3",
  "4",
  "8",
  "16",
  "32",
  "60",
  "120",
  "240",
  "360",
  "3600",
];

function playbackSpeedNumber(playbackSpeed) {
  const numericSpeed = Number(playbackSpeed || 1);
  return Number.isFinite(numericSpeed) && numericSpeed > 0 ? numericSpeed : 1;
}

export function nativePlaybackRateForSpeed(playbackSpeed) {
  return Math.min(playbackSpeedNumber(playbackSpeed), MAX_NATIVE_PLAYBACK_RATE);
}

export function isHighSpeedPlayback(playbackSpeed) {
  return playbackSpeedNumber(playbackSpeed) > MAX_NATIVE_PLAYBACK_RATE;
}

export function getSegmentEndMs(segments, index, sessionEndMs) {
  const segment = segments[index];
  if (!segment) return Number(sessionEndMs || 0);
  if (segment.finished_at !== null && segment.finished_at !== undefined) {
    return Number(segment.finished_at || segment.started_at || 0);
  }
  const next = segments[index + 1];
  if (next) return Number(next.started_at || segment.started_at || 0);
  return Number(sessionEndMs || segment.started_at || 0);
}

export function sliderValueFromSegmentIndex(segmentIndex) {
  return Math.max(1, Number(segmentIndex || 0) + 1);
}

export function segmentIndexFromSliderValue(sliderValue, segmentCount) {
  if (segmentCount <= 0) return -1;
  const normalized = Math.round(Number(sliderValue || 1));
  return Math.max(0, Math.min(segmentCount - 1, normalized - 1));
}

function normalizeTimelineMs(segments, timelineMs, sessionEndMs, loop) {
  const firstStartedAt = Number(segments[0]?.started_at || 0);
  const finalTimelineMs = getSegmentEndMs(segments, segments.length - 1, sessionEndMs);

  if (loop && finalTimelineMs > firstStartedAt) {
    const durationMs = finalTimelineMs - firstStartedAt;
    const wrappedOffset = (((timelineMs - firstStartedAt) % durationMs) + durationMs) % durationMs;
    return {
      ended: false,
      timelineMs: firstStartedAt + wrappedOffset,
    };
  }

  if (timelineMs >= finalTimelineMs) {
    return {
      ended: true,
      timelineMs: finalTimelineMs,
    };
  }

  return {
    ended: false,
    timelineMs: Math.max(firstStartedAt, timelineMs),
  };
}

export function locatePlaybackPosition(segments, timelineMs, sessionEndMs, loop = false) {
  if (!segments.length) {
    return {
      ended: true,
      segmentIndex: -1,
      offsetSeconds: 0,
      timelineMs: 0,
    };
  }

  const normalized = normalizeTimelineMs(segments, timelineMs, sessionEndMs, loop);
  const resolvedTimelineMs = normalized.timelineMs;

  let segmentIndex = segments.length - 1;
  for (let index = 0; index < segments.length; index += 1) {
    const segmentEndMs = getSegmentEndMs(segments, index, sessionEndMs);
    if (resolvedTimelineMs < segmentEndMs || index === segments.length - 1) {
      segmentIndex = index;
      break;
    }
  }

  const segmentStartedAt = Number(segments[segmentIndex]?.started_at || 0);
  const segmentEndMs = getSegmentEndMs(segments, segmentIndex, sessionEndMs);
  const offsetSeconds = Math.max(0, Math.min((resolvedTimelineMs - segmentStartedAt) / 1000, (segmentEndMs - segmentStartedAt) / 1000));

  return {
    ended: normalized.ended,
    segmentIndex,
    offsetSeconds,
    timelineMs: resolvedTimelineMs,
  };
}

export function advanceHighSpeedPosition({
  segments,
  currentTimelineMs,
  playbackSpeed,
  elapsedMs,
  sessionEndMs,
  loop,
}) {
  const targetTimelineMs = Number(currentTimelineMs || 0) + Math.max(0, Number(elapsedMs || 0)) * playbackSpeedNumber(playbackSpeed);
  return locatePlaybackPosition(segments, targetTimelineMs, sessionEndMs, loop);
}

export function applyPlaybackPreferences(videoElement, playbackSpeed) {
  if (!videoElement) return;
  const numericSpeed = nativePlaybackRateForSpeed(playbackSpeed);
  videoElement.defaultPlaybackRate = numericSpeed;
  videoElement.playbackRate = numericSpeed;
  videoElement.loop = false;
}

export function armPlaybackPreferenceSync(videoElement, getPlaybackSpeed) {
  if (!videoElement) return () => {};

  videoElement[playbackSyncKey]?.cleanup?.();

  const sync = () => applyPlaybackPreferences(videoElement, getPlaybackSpeed());
  const listeners = [];

  for (const eventName of ["loadedmetadata", "canplay", "play"]) {
    const handler = () => sync();
    videoElement.addEventListener?.(eventName, handler);
    listeners.push([eventName, handler]);
  }

  const cleanup = () => {
    for (const [eventName, handler] of listeners) {
      videoElement.removeEventListener?.(eventName, handler);
    }
    if (videoElement[playbackSyncKey]?.cleanup === cleanup) {
      delete videoElement[playbackSyncKey];
    }
  };

  videoElement[playbackSyncKey] = { cleanup };
  sync();
  return cleanup;
}
