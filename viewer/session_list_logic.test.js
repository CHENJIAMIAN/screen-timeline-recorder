const test = require("node:test");
const assert = require("node:assert/strict");

const {
  dayKeyFromTimestamp,
  listAvailableSessionDays,
  filterSessionsByDay,
  groupSessionsByDay,
} = require("./session_list_logic.js");

test("builds a stable day key from timestamps", () => {
  assert.equal(dayKeyFromTimestamp(Date.UTC(2026, 3, 14, 8, 30, 0)), "2026-04-14");
});

test("lists distinct days in descending order", () => {
  const days = listAvailableSessionDays([
    { started_at: new Date("2026-04-13T20:00:00+08:00").getTime() },
    { started_at: new Date("2026-04-14T09:00:00+08:00").getTime() },
    { started_at: new Date("2026-04-14T10:00:00+08:00").getTime() },
  ]);

  assert.deepEqual(days, ["2026-04-14", "2026-04-13"]);
});

test("filters sessions by a selected day", () => {
  const sessions = [
    { session_id: "a", started_at: Date.UTC(2026, 3, 14, 9, 0, 0) },
    { session_id: "b", started_at: Date.UTC(2026, 3, 13, 9, 0, 0) },
  ];

  assert.deepEqual(
    filterSessionsByDay(sessions, "2026-04-14").map((session) => session.session_id),
    ["a"]
  );
  assert.equal(filterSessionsByDay(sessions, "all").length, 2);
});

test("groups sessions by day and totals their sizes", () => {
  const groups = groupSessionsByDay([
    { session_id: "a", started_at: Date.UTC(2026, 3, 14, 9, 0, 0), total_bytes: 10 },
    { session_id: "b", started_at: Date.UTC(2026, 3, 14, 11, 0, 0), total_bytes: 20 },
    { session_id: "c", started_at: Date.UTC(2026, 3, 13, 9, 0, 0), total_bytes: 5 },
  ]);

  assert.equal(groups.length, 2);
  assert.equal(groups[0].dayKey, "2026-04-14");
  assert.equal(groups[0].sessionCount, 2);
  assert.equal(groups[0].totalBytes, 30);
  assert.deepEqual(
    groups[0].sessions.map((session) => session.session_id),
    ["a", "b"]
  );
});
