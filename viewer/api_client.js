function buildUrl(path, sessionId = null) {
  const url = new URL(path, window.location.origin);
  if (sessionId) {
    url.searchParams.set("session_id", sessionId);
  }
  return url.toString();
}

async function fetchJson(path, sessionId = null) {
  const response = await fetch(buildUrl(path, sessionId));
  if (!response.ok) {
    let message = `request failed (${response.status})`;
    const contentType = response.headers.get("content-type") || "";
    try {
      if (contentType.includes("application/json")) {
        const payload = await response.json();
        if (payload?.error) {
          message = payload.error;
        }
      } else {
        const text = await response.text();
        if (text) {
          message = text;
        }
      }
    } catch {
      // Keep the default message when the error payload cannot be parsed.
    }
    const error = new Error(message);
    error.status = response.status;
    throw error;
  }
  return response.json();
}

async function fetchControl(action, sessionId = null) {
  return fetchJson(`/api/control?action=${encodeURIComponent(action)}`, sessionId);
}

async function saveAutostart(settings) {
  const query = new URLSearchParams({
    enabled: settings.enabled ? "1" : "0",
    start_on_login: settings.start_on_login ? "1" : "0",
    delay_seconds: String(settings.delay_seconds ?? 0),
    output_dir: settings.output_dir ?? "",
  });
  return fetchJson(`/api/autostart/save?${query.toString()}`);
}

async function saveRecordingSettings(settings) {
  const query = new URLSearchParams({
    sampling_interval_ms: String(settings.sampling_interval_ms ?? 100),
    working_scale: String(settings.working_scale ?? 1),
    burn_in_enabled: settings.burn_in_enabled ? "1" : "0",
  });
  return fetchJson(`/api/recording-settings/save?${query.toString()}`);
}

export {
  buildUrl,
  fetchControl,
  fetchJson,
  saveAutostart,
  saveRecordingSettings,
};
