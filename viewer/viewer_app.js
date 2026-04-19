import { computed, onMounted, reactive, ref, watch } from "./vendor/vue.esm-browser.prod.js";
import { buildUrl, fetchControl, fetchJson, saveAutostart, saveRecordingSettings } from "./api_client.js";
import {
  dayKeyFromTimestamp,
  formatBytes,
  formatClockTime,
  formatClockTimeWithDate,
  formatDayLabel,
  formatElapsed,
} from "./formatters.js";
import { resolveLanguage, translate } from "./i18n.js";

function getSegmentEnd(segments, index, sessionEndMs) {
  const segment = segments[index];
  if (!segment) return Number(sessionEndMs || 0);
  if (segment.finished_at !== null && segment.finished_at !== undefined) {
    return Number(segment.finished_at || segment.started_at || 0);
  }
  const next = segments[index + 1];
  if (next) return Number(next.started_at || segment.started_at || 0);
  return Number(sessionEndMs || segment.started_at || 0);
}

export function createViewerApp() {
  return {
    setup() {
      const videoPlayer = ref(null);
      const state = reactive({
        sessions: [],
        session: null,
        liveStatus: null,
        videoSegments: [],
        activeSegmentIndex: -1,
        autostart: null,
        recordingSettings: null,
        sessionFilter: "all",
        currentSessionId: new URLSearchParams(window.location.search).get("session_id"),
        languagePreference:
          new URLSearchParams(window.location.search).get("lang") ||
          window.localStorage.getItem("viewerLanguage") ||
          "auto",
        language: "en",
        playbackSpeed: "1",
        playbackLoop: false,
        status: "",
        autostartFeedback: "",
        recordingFeedback: "",
      });

      const t = (key, vars = {}) => translate(state.language, key, vars);

      const sessionGroups = computed(() => {
        const filtered =
          state.sessionFilter === "all"
            ? state.sessions
            : state.sessions.filter((session) => dayKeyFromTimestamp(session.started_at) === state.sessionFilter);
        const map = new Map();
        filtered.forEach((session) => {
          const dayKey = dayKeyFromTimestamp(session.started_at || session.last_activity_at || 0);
          if (!map.has(dayKey)) {
            map.set(dayKey, { dayKey, sessions: [], totalBytes: 0 });
          }
          const group = map.get(dayKey);
          group.sessions.push(session);
          group.totalBytes += Number(session.total_bytes || 0);
        });
        return Array.from(map.values()).sort((a, b) => b.dayKey.localeCompare(a.dayKey));
      });

      const availableDays = computed(() =>
        Array.from(new Set(state.sessions.map((session) => dayKeyFromTimestamp(session.started_at)))).sort((a, b) =>
          b.localeCompare(a)
        )
      );

      const sessionEndMs = computed(() => {
        if (!state.session) return 0;
        if (state.session.finished_at !== null && state.session.finished_at !== undefined) {
          return Number(state.session.finished_at);
        }
        if (state.liveStatus?.stats?.finished_at) {
          return Number(state.liveStatus.stats.finished_at);
        }
        return Number(state.session.started_at || 0);
      });

      const currentSegment = computed(() => state.videoSegments[state.activeSegmentIndex] || null);
      const activeSegmentBadge = computed(() =>
        t("clipBadge", { current: Math.max(0, state.activeSegmentIndex + 1), total: state.videoSegments.length })
      );
      const segmentTitle = computed(() =>
        currentSegment.value
          ? t("clipTitle", { current: state.activeSegmentIndex + 1, total: state.videoSegments.length })
          : t("noVideoLoaded")
      );
      const segmentRange = computed(() => {
        if (!currentSegment.value) return t("noVideoLoadedSubtitle");
        const start = formatClockTime(currentSegment.value.started_at, state.language);
        const end = formatClockTime(
          getSegmentEnd(state.videoSegments, state.activeSegmentIndex, sessionEndMs.value),
          state.language
        );
        return t("clipRange", { start, end });
      });
      const currentSessionSummary = computed(() => {
        if (!state.session) return t("loadingSession");
        return t("sessionSummary", {
          time: formatClockTimeWithDate(state.session.started_at, state.language),
          width: state.session.working_width,
          height: state.session.working_height,
        });
      });
      const currentStatusSummary = computed(() => {
        if (!state.session) return t("heartbeatWaiting");
        return t("statusSummaryVideo", {
          width: state.session.working_width,
          height: state.session.working_height,
          segments: state.videoSegments.length,
          size: formatBytes(state.sessions.find((entry) => entry.session_id === state.currentSessionId)?.total_bytes || 0),
          duration: formatElapsed(sessionEndMs.value - Number(state.session.started_at || 0), state.language),
        });
      });

      function applyLanguage() {
        state.language = resolveLanguage(state.languagePreference);
        document.documentElement.lang = state.language === "zh" ? "zh-CN" : "en";
        document.title = t("appTitle");
      }

      function updateUrl() {
        const url = new URL(window.location.href);
        if (state.currentSessionId) url.searchParams.set("session_id", state.currentSessionId);
        if (state.languagePreference) url.searchParams.set("lang", state.languagePreference);
        window.history.replaceState({}, "", url);
      }

      function applyPlaybackPreferences() {
        if (!videoPlayer.value) return;
        videoPlayer.value.playbackRate = Number(state.playbackSpeed || 1);
        videoPlayer.value.loop = false;
      }

      async function loadSessions() {
        state.sessions = await fetchJson("/api/sessions");
        if (!state.currentSessionId && state.sessions.length > 0) {
          state.currentSessionId = state.sessions[0].session_id;
        }
      }

      async function loadVideoSegments() {
        if (!state.currentSessionId) {
          state.videoSegments = [];
          state.activeSegmentIndex = -1;
          return;
        }
        state.videoSegments = await fetchJson("/api/segments", state.currentSessionId);
        state.activeSegmentIndex = state.videoSegments.length > 0 ? 0 : -1;
        await loadActiveSegment(false);
      }

      async function loadSession() {
        if (!state.currentSessionId) {
          await loadSessions();
        }
        if (!state.currentSessionId) {
          state.session = null;
          state.liveStatus = null;
          state.status = t("noSessionsSubtitle");
          return;
        }
        state.session = await fetchJson("/api/session", state.currentSessionId);
        state.liveStatus = await fetchJson("/api/status", state.currentSessionId).catch(() => null);
        await loadVideoSegments();
      }

      async function loadAutostartSettings() {
        state.autostart = await fetchJson("/api/autostart");
      }

      async function loadRecordingConfig() {
        state.recordingSettings = await fetchJson("/api/recording-settings");
      }

      async function refreshAll() {
        await loadSessions();
        await loadSession();
        await loadAutostartSettings();
        await loadRecordingConfig();
        state.status = t("ready");
      }

      async function loadActiveSegment(autoplay) {
        const segment = currentSegment.value;
        if (!segment || !videoPlayer.value) {
          if (videoPlayer.value) {
            videoPlayer.value.removeAttribute("src");
            videoPlayer.value.load();
          }
          return;
        }
        videoPlayer.value.src = buildUrl(`/${segment.relative_path}`, state.currentSessionId);
        videoPlayer.value.load();
        applyPlaybackPreferences();
        if (autoplay) {
          await videoPlayer.value.play().catch(() => {});
        }
      }

      async function selectSession(sessionId) {
        state.currentSessionId = sessionId;
        updateUrl();
        await loadSession();
      }

      async function startRecording() {
        const response = await fetchControl("start");
        state.status = t("startDone");
        if (response.session_id) {
          state.currentSessionId = response.session_id;
          updateUrl();
          await waitForSession(response.session_id);
        }
      }

      async function waitForSession(sessionId) {
        for (let attempt = 0; attempt < 10; attempt += 1) {
          await loadSessions();
          if (state.sessions.some((entry) => entry.session_id === sessionId)) {
            await selectSession(sessionId);
            return;
          }
          await new Promise((resolve) => window.setTimeout(resolve, 400));
        }
      }

      function activeControlSessionId() {
        const active = state.sessions.find((session) => {
          const live = session.status?.state || "unknown";
          return live === "running" || live === "paused";
        });
        return active?.session_id || state.currentSessionId;
      }

      async function stopRecording() {
        const sessionId = activeControlSessionId();
        if (!sessionId) return;
        await fetchControl("stop", sessionId);
        state.status = t("stopDone");
        await refreshAll();
      }

      async function deleteSession(sessionId) {
        await fetchControl("delete", sessionId);
        state.status = t("deleteDone");
        await refreshAll();
      }

      async function deleteSessionsByIds(ids, doneKey) {
        for (const id of ids) {
          await fetchControl("delete", id);
        }
        state.status = t(doneKey);
        await refreshAll();
      }

      async function deleteDaySessions() {
        const ids = state.sessions
          .filter((session) =>
            state.sessionFilter === "all" ? true : dayKeyFromTimestamp(session.started_at) === state.sessionFilter
          )
          .map((session) => session.session_id);
        await deleteSessionsByIds(ids, "deleteDayDone");
      }

      async function deleteAllSessions() {
        await deleteSessionsByIds(
          state.sessions.map((session) => session.session_id),
          "deleteAllDone"
        );
      }

      async function saveAutostartConfig() {
        const settings = state.autostart?.settings;
        if (!settings) return;
        state.autostart = await saveAutostart(settings);
        state.autostartFeedback = t("autostartSaved");
      }

      async function saveRecordingConfig() {
        if (!state.recordingSettings) return;
        state.recordingSettings = await saveRecordingSettings(state.recordingSettings);
        state.recordingFeedback = t("recordingSaved");
      }

      async function previousClip() {
        if (state.activeSegmentIndex <= 0) return;
        state.activeSegmentIndex -= 1;
        await loadActiveSegment(false);
      }

      async function nextClip(autoplay = false) {
        if (state.activeSegmentIndex >= state.videoSegments.length - 1) {
          if (state.playbackLoop && state.videoSegments.length > 0) {
            state.activeSegmentIndex = 0;
            await loadActiveSegment(autoplay);
          }
          return;
        }
        state.activeSegmentIndex += 1;
        await loadActiveSegment(autoplay);
      }

      function sessionStateLabel(session) {
        const raw = session.status?.state || (session.finished_at ? "stopped" : "unknown");
        if (raw === "running") return t("running");
        if (raw === "paused") return t("paused");
        if (raw === "stopped") return t("stopped");
        return t("unknown");
      }

      watch(
        () => state.languagePreference,
        () => {
          window.localStorage.setItem("viewerLanguage", state.languagePreference);
          applyLanguage();
          updateUrl();
        }
      );

      watch(
        () => state.playbackSpeed,
        () => applyPlaybackPreferences()
      );

      onMounted(async () => {
        applyLanguage();
        state.status = t("loadingSession");
        await refreshAll();
      });

      return {
        activeSegmentBadge,
        applyPlaybackPreferences,
        availableDays,
        currentSegment,
        currentSessionSummary,
        currentStatusSummary,
        deleteAllSessions,
        deleteDaySessions,
        deleteSession,
        formatBytes,
        formatClockTime,
        formatClockTimeWithDate,
        formatDayLabel,
        formatElapsed,
        loadSession,
        loadVideoSegments,
        nextClip,
        previousClip,
        refreshAll,
        saveAutostartConfig,
        saveRecordingConfig,
        segmentRange,
        segmentTitle,
        selectSession,
        sessionGroups,
        sessionStateLabel,
        startRecording,
        state,
        stopRecording,
        t,
        videoPlayer,
      };
    },
    template: `
      <main class="app">
        <header class="toolbar">
          <div class="session-meta">
            <div id="viewer-title" class="title">{{ t('appTitle') }}</div>
            <div id="session-info" class="subtitle">{{ currentSessionSummary }}</div>
          </div>
          <label class="field language-field">
            <span id="language-label">{{ t('language') }}</span>
            <select id="language-select" v-model="state.languagePreference">
              <option value="auto">{{ t('auto') }}</option>
              <option value="en">{{ t('english') }}</option>
              <option value="zh">{{ t('chinese') }}</option>
            </select>
          </label>
        </header>

        <section id="quickstart" class="panel quickstart">
          <div class="panel-header">
            <div id="quickstart-title" class="title">{{ t('howToUse') }}</div>
            <div id="quickstart-subtitle" class="subtitle">{{ t('howToUseSubtitle') }}</div>
          </div>
          <div class="quickstart-steps">
            <article class="quickstart-step">
              <span class="quickstart-step-number">1</span>
              <div>
                <div id="quickstart-step1-title" class="quickstart-step-title">{{ t('step1Title') }}</div>
                <div id="quickstart-step1-body" class="quickstart-step-body">{{ t('step1Body') }}</div>
              </div>
            </article>
            <article class="quickstart-step">
              <span class="quickstart-step-number">2</span>
              <div>
                <div id="quickstart-step2-title" class="quickstart-step-title">{{ t('step2Title') }}</div>
                <div id="quickstart-step2-body" class="quickstart-step-body">{{ t('step2Body') }}</div>
              </div>
            </article>
            <article class="quickstart-step">
              <span class="quickstart-step-number">3</span>
              <div>
                <div id="quickstart-step3-title" class="quickstart-step-title">{{ t('step3Title') }}</div>
                <div id="quickstart-step3-body" class="quickstart-step-body">{{ t('step3Body') }}</div>
              </div>
            </article>
          </div>
        </section>

        <section class="panel live-status">
          <div class="live-status-header">
            <div id="live-status-title" class="title">{{ t('liveStatus') }}</div>
            <div id="recording-badge" class="recording-badge">{{ state.liveStatus?.state || t('checking') }}</div>
          </div>
          <div class="live-status-actions">
            <button id="control-refresh" class="ghost" type="button" @click="refreshAll">{{ t('refresh') }}</button>
            <button id="control-start" class="primary" type="button" @click="startRecording">{{ t('startRecording') }}</button>
            <button id="control-stop" type="button" @click="stopRecording">{{ t('stopRecording') }}</button>
          </div>
          <div id="status-summary" class="status-summary">{{ currentStatusSummary }}</div>
        </section>

        <section id="autostart-settings" class="panel">
          <div class="panel-header split">
            <div>
              <div id="autostart-title" class="title">{{ t('autostartTitle') }}</div>
              <div id="autostart-subtitle" class="subtitle">{{ t('autostartSubtitle') }}</div>
            </div>
            <div id="autostart-state" class="badge">{{ state.autostart?.enabled ? t('running') : t('stopped') }}</div>
          </div>
          <div class="grid" v-if="state.autostart?.settings">
            <label class="toggle">
              <input id="autostart-enabled" type="checkbox" v-model="state.autostart.settings.enabled" />
              <span id="autostart-enabled-label">{{ t('autostartEnabled') }}</span>
            </label>
            <label class="toggle">
              <input id="autostart-login" type="checkbox" v-model="state.autostart.settings.start_on_login" />
              <span id="autostart-login-label">{{ t('autostartLogin') }}</span>
            </label>
            <label class="field">
              <span id="autostart-delay-label">{{ t('autostartDelay') }}</span>
              <input id="autostart-delay" type="number" min="0" max="3600" step="1" v-model="state.autostart.settings.delay_seconds" />
            </label>
            <label class="field full-row">
              <span id="autostart-output-label">{{ t('autostartOutput') }}</span>
              <input id="autostart-output-dir" type="text" v-model="state.autostart.settings.output_dir" />
            </label>
          </div>
          <div id="autostart-note" class="note">{{ t('autostartNote') }}</div>
          <div id="autostart-feedback" class="feedback" v-if="state.autostartFeedback">{{ state.autostartFeedback }}</div>
          <div class="actions">
            <button id="autostart-refresh" class="ghost" type="button" @click="refreshAll">{{ t('refresh') }}</button>
            <button id="autostart-save" type="button" @click="saveAutostartConfig">{{ t('save') }}</button>
          </div>
        </section>

        <section id="recording-settings" class="panel">
          <div class="panel-header">
            <div id="recording-settings-title" class="title">{{ t('videoRecording') }}</div>
            <div id="recording-settings-subtitle" class="subtitle">{{ t('videoRecordingSubtitle') }}</div>
          </div>
          <div class="grid" v-if="state.recordingSettings">
            <label class="field">
              <span id="recording-sampling-interval-label">{{ t('frameInterval') }}</span>
              <input id="recording-sampling-interval" type="number" min="100" max="5000" step="50" v-model="state.recordingSettings.sampling_interval_ms" />
            </label>
            <label class="field">
              <span id="recording-working-scale-label">{{ t('captureScale') }}</span>
              <input id="recording-working-scale" type="number" min="0.1" max="1" step="0.05" v-model="state.recordingSettings.working_scale" />
            </label>
            <label class="toggle">
              <input id="recording-burn-in-enabled" type="checkbox" v-model="state.recordingSettings.burn_in_enabled" />
              <span id="recording-burn-in-enabled-label">{{ t('burnIn') }}</span>
            </label>
          </div>
          <div id="recording-settings-note" class="note">{{ t('recordingNote') }}</div>
          <div id="recording-settings-feedback" class="feedback" v-if="state.recordingFeedback">{{ state.recordingFeedback }}</div>
          <div class="actions">
            <button id="recording-refresh" class="ghost" type="button" @click="refreshAll">{{ t('refresh') }}</button>
            <button id="recording-save" type="button" @click="saveRecordingConfig">{{ t('save') }}</button>
          </div>
        </section>

        <section class="panel viewer-panel">
          <div class="panel-header split">
            <div>
              <div id="viewer-panel-title" class="title">{{ t('playback') }}</div>
              <div id="viewer-panel-subtitle" class="subtitle">{{ t('playbackSubtitle') }}</div>
            </div>
            <div id="viewer-segment-badge" class="badge">{{ activeSegmentBadge }}</div>
          </div>
          <video id="video-player" ref="videoPlayer" controls playsinline muted @ended="nextClip(true)"></video>
          <div id="viewer-player-panel" class="viewer-player-panel">
            <div class="viewer-player-meta">
              <div id="viewer-segment-title" class="viewer-player-title">{{ segmentTitle }}</div>
              <div id="viewer-segment-range" class="viewer-player-range">{{ segmentRange }}</div>
            </div>
            <div class="viewer-player-actions">
              <button id="segment-prev" class="ghost" type="button" @click="previousClip">{{ t('previousClip') }}</button>
              <button id="segment-next" class="ghost" type="button" @click="nextClip(false)">{{ t('nextClip') }}</button>
              <label class="field compact-field">
                <span id="speed-label">{{ t('speed') }}</span>
                <select id="playback-speed" v-model="state.playbackSpeed">
                  <option value="0.25">0.25x</option>
                  <option value="0.5">0.5x</option>
                  <option value="0.75">0.75x</option>
                  <option value="1">1x</option>
                  <option value="1.25">1.25x</option>
                  <option value="1.5">1.5x</option>
                  <option value="2">2x</option>
                  <option value="3">3x</option>
                  <option value="4">4x</option>
                  <option value="8">8x</option>
                </select>
              </label>
              <label class="toggle">
                <input id="playback-loop" type="checkbox" v-model="state.playbackLoop" />
                <span id="playback-loop-label">{{ t('loopPlayback') }}</span>
              </label>
            </div>
          </div>
        </section>

        <section id="session-list" class="panel">
          <div class="panel-header split">
            <div>
              <div id="recent-sessions-title" class="title">{{ t('recentSessions') }}</div>
              <div id="recent-sessions-subtitle" class="subtitle">{{ t('recentSessionsSubtitle') }}</div>
            </div>
            <div class="session-list-actions">
              <label class="field compact-field">
                <span id="session-filter-label">{{ t('dateFilter') }}</span>
                <select id="session-filter" v-model="state.sessionFilter">
                  <option value="all">{{ t('allDates') }}</option>
                  <option v-for="day in availableDays" :key="day" :value="day">{{ formatDayLabel(day, state.language) }}</option>
                </select>
              </label>
              <button id="delete-day-sessions" class="ghost danger" type="button" @click="deleteDaySessions">{{ t('deleteDay') }}</button>
              <button id="delete-all-sessions" class="ghost danger" type="button" @click="deleteAllSessions">{{ t('deleteAll') }}</button>
              <button id="refresh-sessions" class="ghost" type="button" @click="refreshAll">{{ t('refresh') }}</button>
            </div>
          </div>
          <div id="session-list-grid" class="session-list-grid" v-if="sessionGroups.length > 0">
            <section class="session-day-group" v-for="group in sessionGroups" :key="group.dayKey">
              <div class="session-day-header">
                <div>
                  <div class="session-day-title">{{ formatDayLabel(group.dayKey, state.language) }}</div>
                  <div class="session-day-summary">{{ group.sessions.length }} | {{ formatBytes(group.totalBytes) }}</div>
                </div>
              </div>
              <div class="session-day-grid">
                <article class="session-card" v-for="session in group.sessions" :key="session.session_id" :class="{ current: session.session_id === state.currentSessionId }">
                  <button class="session-card-open" type="button" @click="selectSession(session.session_id)">
                    <div class="session-card-title">{{ formatClockTimeWithDate(session.started_at, state.language) }}</div>
                    <div class="session-card-subtitle">{{ sessionStateLabel(session) }}</div>
                    <div class="session-card-body">
                      <span class="session-card-duration">{{ t('duration') }} {{ formatElapsed((session.finished_at || session.last_activity_at) - session.started_at, state.language) }}</span>
                      <span class="session-card-size">{{ t('size') }} {{ formatBytes(session.total_bytes) }}</span>
                    </div>
                  </button>
                  <button class="session-card-delete ghost danger" type="button" @click="deleteSession(session.session_id)">{{ t('delete') }}</button>
                </article>
              </div>
            </section>
          </div>
          <div class="session-card placeholder" v-else>
            <div class="session-card-title">{{ t('noSessions') }}</div>
            <div class="session-card-subtitle">{{ t('noSessionsSubtitle') }}</div>
          </div>
        </section>

        <footer class="status">
          <span id="status">{{ state.status }}</span>
        </footer>
      </main>
    `,
  };
}
