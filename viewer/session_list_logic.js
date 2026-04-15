(function (root, factory) {
  const api = factory();
  if (typeof module !== "undefined" && module.exports) {
    module.exports = api;
  }
  root.ScreenTimelineSessionListLogic = api;
})(typeof globalThis !== "undefined" ? globalThis : this, () => {
  function dayKeyFromTimestamp(timestampMs) {
    const date = new Date(Number(timestampMs) || 0);
    const year = date.getFullYear();
    const month = String(date.getMonth() + 1).padStart(2, "0");
    const day = String(date.getDate()).padStart(2, "0");
    return `${year}-${month}-${day}`;
  }

  function listAvailableSessionDays(sessions) {
    return Array.from(
      new Set((sessions || []).map((session) => dayKeyFromSession(session)))
    ).sort((left, right) => right.localeCompare(left));
  }

  function filterSessionsByDay(sessions, selectedDay) {
    if (!selectedDay || selectedDay === "all") {
      return [...(sessions || [])];
    }
    return (sessions || []).filter((session) => dayKeyFromSession(session) === selectedDay);
  }

  function groupSessionsByDay(sessions) {
    const groups = new Map();
    for (const session of sessions || []) {
      const dayKey = dayKeyFromSession(session);
      if (!groups.has(dayKey)) {
        groups.set(dayKey, {
          dayKey,
          sessions: [],
          totalBytes: 0,
          sessionCount: 0,
        });
      }
      const group = groups.get(dayKey);
      group.sessions.push(session);
      group.totalBytes += Number(session.total_bytes || 0);
      group.sessionCount += 1;
    }

    return Array.from(groups.values()).sort((left, right) => right.dayKey.localeCompare(left.dayKey));
  }

  function dayKeyFromSession(session) {
    return dayKeyFromTimestamp(session.started_at || session.last_activity_at || 0);
  }

  return {
    dayKeyFromTimestamp,
    listAvailableSessionDays,
    filterSessionsByDay,
    groupSessionsByDay,
  };
});
