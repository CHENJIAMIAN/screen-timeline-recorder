function formatBytes(bytes) {
  const value = Number(bytes || 0);
  if (value < 1024) return `${value} B`;
  if (value < 1024 * 1024) return `${(value / 1024).toFixed(1)} KB`;
  if (value < 1024 * 1024 * 1024) return `${(value / (1024 * 1024)).toFixed(1)} MB`;
  return `${(value / (1024 * 1024 * 1024)).toFixed(1)} GB`;
}

function formatElapsed(durationMs, language) {
  const totalMs = Math.max(0, Number(durationMs || 0));
  const totalSeconds = Math.floor(totalMs / 1000);
  const hours = Math.floor(totalSeconds / 3600);
  const minutes = Math.floor((totalSeconds % 3600) / 60);
  const seconds = totalSeconds % 60;

  if (hours > 0) {
    return language === "zh"
      ? `${hours}小时 ${minutes}分 ${seconds}秒`
      : `${hours}h ${minutes}m ${seconds}s`;
  }
  if (minutes > 0) {
    return language === "zh" ? `${minutes}分 ${seconds}秒` : `${minutes}m ${seconds}s`;
  }
  if (totalSeconds > 0) {
    return language === "zh" ? `${totalSeconds}秒` : `${totalSeconds}s`;
  }
  return language === "zh" ? `${totalMs}毫秒` : `${totalMs}ms`;
}

function formatClockTime(timestampMs, language) {
  const value = Number(timestampMs || 0);
  if (!Number.isFinite(value) || value <= 0) return String(timestampMs ?? "");
  return new Intl.DateTimeFormat(language === "zh" ? "zh-CN" : "en-US", {
    month: "2-digit",
    day: "2-digit",
    hour: "2-digit",
    minute: "2-digit",
    second: "2-digit",
    hour12: false,
  }).format(new Date(value));
}

function formatClockTimeWithDate(timestampMs, language) {
  const value = Number(timestampMs || 0);
  if (!Number.isFinite(value) || value <= 0) return String(timestampMs ?? "");
  return new Intl.DateTimeFormat(language === "zh" ? "zh-CN" : "en-US", {
    year: "numeric",
    month: "2-digit",
    day: "2-digit",
    hour: "2-digit",
    minute: "2-digit",
    second: "2-digit",
    hour12: false,
  }).format(new Date(value));
}

function dayKeyFromTimestamp(timestampMs) {
  const date = new Date(Number(timestampMs || 0));
  return `${date.getFullYear()}-${String(date.getMonth() + 1).padStart(2, "0")}-${String(date.getDate()).padStart(2, "0")}`;
}

function formatDayLabel(dayKey, language) {
  const [year, month, day] = String(dayKey || "").split("-");
  if (!year || !month || !day) return dayKey;
  return language === "zh" ? `${year}年${month}月${day}日` : `${year}-${month}-${day}`;
}

export {
  dayKeyFromTimestamp,
  formatBytes,
  formatClockTime,
  formatClockTimeWithDate,
  formatDayLabel,
  formatElapsed,
};
