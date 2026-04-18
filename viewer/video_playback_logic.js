(function (root, factory) {
  const api = factory();
  if (typeof module !== "undefined" && module.exports) {
    module.exports = api;
  }
  root.ScreenTimelineVideoPlaybackLogic = api;
})(typeof globalThis !== "undefined" ? globalThis : this, () => {
  function getVideoSegmentEndMs(segments, index, sessionEndMs) {
    const segment = (segments || [])[index];
    if (!segment) {
      return Number(sessionEndMs || 0);
    }

    if (segment.finished_at !== null && segment.finished_at !== undefined) {
      return Number(segment.finished_at || segment.started_at || 0);
    }

    const next = (segments || [])[index + 1];
    if (next) {
      return Number(next.started_at || segment.started_at || 0);
    }

    return Number(sessionEndMs || segment.started_at || 0);
  }

  function findVideoSegmentIndex(segments, timestampMs, sessionEndMs) {
    const allSegments = segments || [];
    if (!allSegments.length) {
      return -1;
    }

    const target = Number(timestampMs || 0);
    for (let index = 0; index < allSegments.length; index += 1) {
      const start = Number(allSegments[index].started_at || 0);
      const end = getVideoSegmentEndMs(allSegments, index, sessionEndMs);
      const isLast = index === allSegments.length - 1;
      if (target >= start && (target < end || (isLast && target <= end))) {
        return index;
      }
    }

    return -1;
  }

  function getVideoTargetTimeSeconds(segment, timestampMs) {
    const startedAt = Number(segment && segment.started_at ? segment.started_at : 0);
    const target = Math.max(0, Number(timestampMs || 0) - startedAt);
    return target / 1000;
  }

  function shouldSeekVideo(currentSeconds, targetSeconds, toleranceSeconds = 0.2) {
    if (!Number.isFinite(currentSeconds) || !Number.isFinite(targetSeconds)) {
      return true;
    }
    return Math.abs(currentSeconds - targetSeconds) > toleranceSeconds;
  }

  return {
    findVideoSegmentIndex,
    getVideoSegmentEndMs,
    getVideoTargetTimeSeconds,
    shouldSeekVideo,
  };
});
