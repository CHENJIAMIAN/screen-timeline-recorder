(function (root, factory) {
  const api = factory();
  if (typeof module !== "undefined" && module.exports) {
    module.exports = api;
  }
  root.ScreenTimelineControlLogic = api;
})(typeof globalThis !== "undefined" ? globalThis : this, () => {
  function normalizeStatusState(liveStatus) {
    if (liveStatus && typeof liveStatus.state === "string") {
      return liveStatus.state;
    }

    if (!liveStatus) {
      return "unknown";
    }

    return liveStatus.recording ? "running" : "stopped";
  }

  function isActiveState(liveState) {
    return liveState === "running" || liveState === "paused";
  }

  function resolveControlTarget({ currentSessionId, currentLiveState, sessions }) {
    if (currentSessionId && isActiveState(currentLiveState)) {
      return {
        sessionId: currentSessionId,
        liveState: currentLiveState,
        source: "current",
      };
    }

    const activeSession = (sessions || []).find((session) => {
      const liveState = normalizeStatusState(session.status);
      return isActiveState(liveState);
    });

    if (activeSession) {
      return {
        sessionId: activeSession.session_id,
        liveState: normalizeStatusState(activeSession.status),
        source: "active-session",
      };
    }

    return {
      sessionId: currentSessionId || null,
      liveState: currentLiveState || "unknown",
      source: "fallback",
    };
  }

  function computeControlButtons({ currentSessionId, currentLiveState, sessions }) {
    const target = resolveControlTarget({ currentSessionId, currentLiveState, sessions });
    const activeSessionExists = (sessions || []).some((session) => {
      const liveState = normalizeStatusState(session.status);
      return isActiveState(liveState);
    });

    const buttons = {
      controlSessionId: target.sessionId,
      controlLiveState: target.liveState,
      startDisabled: activeSessionExists,
      pauseDisabled: false,
      resumeDisabled: false,
      stopDisabled: false,
    };

    if (target.liveState === "running") {
      buttons.resumeDisabled = true;
      return buttons;
    }

    if (target.liveState === "paused") {
      buttons.pauseDisabled = true;
      return buttons;
    }

    if (target.liveState === "stopped") {
      buttons.pauseDisabled = true;
      buttons.resumeDisabled = true;
      buttons.stopDisabled = true;
      return buttons;
    }

    return buttons;
  }

  return {
    normalizeStatusState,
    resolveControlTarget,
    computeControlButtons,
  };
});
