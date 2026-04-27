import { computed, onMounted, reactive, ref, watch } from "./vendor/vue.esm-browser.prod.js";
import { buildUrl, fetchControl, fetchJson, saveAutostart, saveRecordingSettings } from "./api_client.js";
import {
  advanceHighSpeedPosition,
  applyPlaybackPreferences as applyVideoPlaybackPreferences,
  armPlaybackPreferenceSync,
  getSegmentEndMs as getVideoSegmentEndMs,
  isHighSpeedPlayback,
  playbackSpeedOptions,
  segmentIndexFromSliderValue,
  sliderValueFromSegmentIndex,
} from "./video_player.js";
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
  return getVideoSegmentEndMs(segments, index, sessionEndMs);
}

export function createViewerApp() {
  return {
    setup() {
      let cleanupPlaybackSync = () => {};
      let turboPlaybackIntervalId = null;
      let turboAnchorTimelineMs = 0;
      let turboAnchorStartedAtMs = 0;
      let turboTickInFlight = false;
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
        isRefreshing: false,
        isSelectingSession: false,
        loadingSessionId: null,
        isStartingRecording: false,
        isStoppingRecording: false,
        isSavingAutostart: false,
        isSavingRecordingSettings: false,
        isDeletingDay: false,
        isDeletingAll: false,
        deletingSessionIds: {},
        status: "",
        controlFeedback: "",
        controlFeedbackVariant: "working",
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
      const activeSegmentSliderValue = computed(() => sliderValueFromSegmentIndex(state.activeSegmentIndex));
      const activeSegmentSliderPercent = computed(() => {
        if (state.videoSegments.length <= 1) return 100;
        return ((activeSegmentSliderValue.value - 1) / (state.videoSegments.length - 1)) * 100;
      });
      const segmentSliderStyle = computed(() => ({
        background: `linear-gradient(90deg, rgba(73, 197, 140, 0.92) 0%, rgba(73, 197, 140, 0.92) ${activeSegmentSliderPercent.value}%, rgba(255, 255, 255, 0.12) ${activeSegmentSliderPercent.value}%, rgba(255, 255, 255, 0.12) 100%)`,
      }));
      const isBusy = computed(
        () =>
          state.isRefreshing ||
          state.isSelectingSession ||
          state.isStartingRecording ||
          state.isStoppingRecording ||
          state.isSavingAutostart ||
          state.isSavingRecordingSettings ||
          state.isDeletingDay ||
          state.isDeletingAll ||
          Object.keys(state.deletingSessionIds).length > 0
      );
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
        else url.searchParams.delete("session_id");
        if (state.languagePreference) url.searchParams.set("lang", state.languagePreference);
        window.history.replaceState({}, "", url);
      }

      function applyPlaybackPreferences() {
        applyPlaybackPreferencesToCurrentPlayer();
      }

      function applyPlaybackPreferencesToCurrentPlayer() {
        if (!videoPlayer.value) return;
        applyVideoPlaybackPreferences(videoPlayer.value, state.playbackSpeed);
      }

      function getCurrentTimelinePositionMs() {
        const segment = currentSegment.value;
        if (!segment || !videoPlayer.value) return 0;
        return Number(segment.started_at || 0) + Math.max(0, Number(videoPlayer.value.currentTime || 0)) * 1000;
      }

      function resetTurboPlaybackAnchor() {
        turboAnchorTimelineMs = getCurrentTimelinePositionMs();
        turboAnchorStartedAtMs = performance.now();
      }

      function stopTurboPlayback() {
        if (turboPlaybackIntervalId !== null) {
          window.clearInterval(turboPlaybackIntervalId);
          turboPlaybackIntervalId = null;
        }
        turboTickInFlight = false;
      }

      async function ensurePlayerReady(player) {
        if (!player || player.readyState >= 1) return;
        await new Promise((resolve) => {
          let resolved = false;
          const finish = () => {
            if (resolved) return;
            resolved = true;
            player.removeEventListener("loadedmetadata", finish);
            player.removeEventListener("canplay", finish);
            player.removeEventListener("error", finish);
            resolve();
          };
          player.addEventListener("loadedmetadata", finish, { once: true });
          player.addEventListener("canplay", finish, { once: true });
          player.addEventListener("error", finish, { once: true });
        });
      }

      async function seekToPlaybackPosition(segmentIndex, offsetSeconds, autoplay) {
        if (!videoPlayer.value) return;
        if (segmentIndex < 0 || !state.videoSegments[segmentIndex]) return;

        if (state.activeSegmentIndex !== segmentIndex) {
          state.activeSegmentIndex = segmentIndex;
          await loadActiveSegment(autoplay, offsetSeconds);
          return;
        }

        if (Math.abs((videoPlayer.value.currentTime || 0) - offsetSeconds) > 0.2) {
          videoPlayer.value.currentTime = offsetSeconds;
        }
        if (autoplay && videoPlayer.value.paused) {
          await videoPlayer.value.play().catch(() => {});
        }
      }

      async function tickTurboPlayback() {
        if (turboTickInFlight || !videoPlayer.value || videoPlayer.value.paused) {
          if (!videoPlayer.value || videoPlayer.value?.paused) {
            stopTurboPlayback();
          }
          return;
        }

        turboTickInFlight = true;
        try {
          const target = advanceHighSpeedPosition({
            segments: state.videoSegments,
            currentTimelineMs: turboAnchorTimelineMs,
            playbackSpeed: state.playbackSpeed,
            elapsedMs: performance.now() - turboAnchorStartedAtMs,
            sessionEndMs: sessionEndMs.value,
            loop: state.playbackLoop,
          });

          if (target.segmentIndex < 0) {
            stopTurboPlayback();
            return;
          }

          await seekToPlaybackPosition(target.segmentIndex, target.offsetSeconds, !target.ended);

          if (target.ended && videoPlayer.value) {
            stopTurboPlayback();
            videoPlayer.value.pause();
          }
        } finally {
          turboTickInFlight = false;
        }
      }

      function syncTurboPlayback() {
        if (!videoPlayer.value || !currentSegment.value) {
          stopTurboPlayback();
          return;
        }

        if (videoPlayer.value.paused || !isHighSpeedPlayback(state.playbackSpeed)) {
          stopTurboPlayback();
          return;
        }

        resetTurboPlaybackAnchor();
        if (turboPlaybackIntervalId !== null) return;

        turboPlaybackIntervalId = window.setInterval(() => {
          void tickTurboPlayback();
        }, 100);
      }

      async function loadSessions() {
        state.sessions = await fetchJson("/api/sessions");
        if (state.sessions.length === 0) {
          state.currentSessionId = null;
          state.session = null;
          state.liveStatus = null;
          state.videoSegments = [];
          state.activeSegmentIndex = -1;
          updateUrl();
          return;
        }
        if (!state.sessions.some((session) => session.session_id === state.currentSessionId)) {
          state.currentSessionId = state.sessions[0].session_id;
          updateUrl();
        }
      }

      async function loadVideoSegments() {
        stopTurboPlayback();
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
        try {
          state.session = await fetchJson("/api/session", state.currentSessionId);
        } catch (error) {
          await loadSessions();
          if (!state.currentSessionId) {
            state.session = null;
            state.liveStatus = null;
            state.status = t("noSessionsSubtitle");
            return;
          }
          state.session = await fetchJson("/api/session", state.currentSessionId);
        }
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
        stopTurboPlayback();
        state.isRefreshing = true;
        state.status = t("refreshing");
        try {
          await loadSessions();
          await loadSession();
          await loadAutostartSettings();
          await loadRecordingConfig();
          state.controlFeedback = "";
          state.controlFeedbackVariant = "working";
          state.status = t("ready");
        } finally {
          state.isRefreshing = false;
        }
      }

      async function loadActiveSegment(autoplay, startOffsetSeconds = 0) {
        const segment = currentSegment.value;
        if (!segment || !videoPlayer.value) {
          stopTurboPlayback();
          cleanupPlaybackSync();
          if (videoPlayer.value) {
            videoPlayer.value.removeAttribute("src");
            videoPlayer.value.load();
          }
          return;
        }
        cleanupPlaybackSync();
        cleanupPlaybackSync = armPlaybackPreferenceSync(videoPlayer.value, () => state.playbackSpeed);
        videoPlayer.value.src = buildUrl(`/${segment.relative_path}`, state.currentSessionId);
        videoPlayer.value.load();
        applyPlaybackPreferencesToCurrentPlayer();
        await ensurePlayerReady(videoPlayer.value);
        if (startOffsetSeconds > 0) {
          videoPlayer.value.currentTime = startOffsetSeconds;
        }
        if (autoplay) {
          await videoPlayer.value.play().catch(() => {});
        }
      }

      async function selectSession(sessionId) {
        stopTurboPlayback();
        state.isSelectingSession = true;
        state.loadingSessionId = sessionId;
        state.status = t("loadingSession");
        try {
          state.currentSessionId = sessionId;
          updateUrl();
          await loadSession();
        } finally {
          state.isSelectingSession = false;
          state.loadingSessionId = null;
        }
      }

      async function startRecording() {
        state.isStartingRecording = true;
        state.controlFeedback = t("startingRecording");
        state.controlFeedbackVariant = "working";
        state.status = t("startingRecording");
        try {
          const response = await fetchControl("start");
          state.status = t("startDone");
          state.controlFeedback = t("startDone");
          state.controlFeedbackVariant = "success";
          if (response.session_id) {
            state.currentSessionId = response.session_id;
            updateUrl();
            await waitForSession(response.session_id);
          }
        } catch (error) {
          state.status = error?.message || t("startPendingFailed");
          state.controlFeedback = state.status;
          state.controlFeedbackVariant = "error";
        } finally {
          state.isStartingRecording = false;
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
        throw new Error(t("startPendingFailed"));
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
        state.isStoppingRecording = true;
        state.controlFeedback = t("stoppingRecording");
        state.controlFeedbackVariant = "working";
        state.status = t("stoppingRecording");
        try {
          await fetchControl("stop", sessionId);
          state.status = t("stopDone");
          state.controlFeedback = t("stopDone");
          state.controlFeedbackVariant = "success";
          await refreshAll();
          state.controlFeedback = t("stopDone");
          state.controlFeedbackVariant = "success";
        } catch (error) {
          state.status = error?.message || t("stopDone");
          state.controlFeedback = state.status;
          state.controlFeedbackVariant = "error";
        } finally {
          state.isStoppingRecording = false;
        }
      }

      async function deleteSession(sessionId) {
        state.deletingSessionIds = { ...state.deletingSessionIds, [sessionId]: true };
        state.status = t("deleting");
        try {
          await fetchControl("delete", sessionId);
          state.status = t("deleteDone");
          await refreshAll();
        } finally {
          const next = { ...state.deletingSessionIds };
          delete next[sessionId];
          state.deletingSessionIds = next;
        }
      }

      async function deleteSessionsByIds(ids, doneKey) {
        const nextDeleting = { ...state.deletingSessionIds };
        ids.forEach((id) => {
          nextDeleting[id] = true;
        });
        state.deletingSessionIds = nextDeleting;
        try {
          for (const id of ids) {
            await fetchControl("delete", id);
          }
          state.status = t(doneKey);
          await refreshAll();
        } finally {
          const next = { ...state.deletingSessionIds };
          ids.forEach((id) => {
            delete next[id];
          });
          state.deletingSessionIds = next;
        }
      }

      async function deleteDaySessions() {
        const ids = state.sessions
          .filter((session) =>
            state.sessionFilter === "all" ? true : dayKeyFromTimestamp(session.started_at) === state.sessionFilter
          )
          .map((session) => session.session_id);
        state.isDeletingDay = true;
        state.status = t("deletingDay");
        try {
          await deleteSessionsByIds(ids, "deleteDayDone");
        } finally {
          state.isDeletingDay = false;
        }
      }

      async function deleteAllSessions() {
        state.isDeletingAll = true;
        state.status = t("deletingAll");
        try {
          await deleteSessionsByIds(
            state.sessions.map((session) => session.session_id),
            "deleteAllDone"
          );
        } finally {
          state.isDeletingAll = false;
        }
      }

      async function saveAutostartConfig() {
        const settings = state.autostart?.settings;
        if (!settings) return;
        state.isSavingAutostart = true;
        try {
          state.autostart = await saveAutostart(settings);
          state.autostartFeedback = t("autostartSaved");
        } finally {
          state.isSavingAutostart = false;
        }
      }

      async function saveRecordingConfig() {
        if (!state.recordingSettings) return;
        state.isSavingRecordingSettings = true;
        try {
          state.recordingSettings = await saveRecordingSettings(state.recordingSettings);
          state.recordingFeedback = t("recordingSaved");
        } finally {
          state.isSavingRecordingSettings = false;
        }
      }

      async function previousClip() {
        stopTurboPlayback();
        if (state.activeSegmentIndex <= 0) return;
        state.activeSegmentIndex -= 1;
        await loadActiveSegment(false);
      }

      async function nextClip(autoplay = false) {
        stopTurboPlayback();
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

      async function selectClipFromSlider(event) {
        const targetIndex = segmentIndexFromSliderValue(event?.target?.value, state.videoSegments.length);
        if (targetIndex < 0 || targetIndex === state.activeSegmentIndex) return;

        stopTurboPlayback();
        state.activeSegmentIndex = targetIndex;
        const autoplay = Boolean(videoPlayer.value && !videoPlayer.value.paused);
        await loadActiveSegment(autoplay);
      }

      function sessionStateLabel(session) {
        const raw = session.status?.state || (session.finished_at ? "stopped" : "unknown");
        if (raw === "running") return t("running");
        if (raw === "paused") return t("paused");
        if (raw === "stopped") return t("stopped");
        return t("unknown");
      }

      function isSessionDeleting(sessionId) {
        return Boolean(state.deletingSessionIds[sessionId]);
      }

      async function handlePlaybackEnded() {
        stopTurboPlayback();
        await nextClip(true);
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
        () => {
          applyPlaybackPreferences();
          syncTurboPlayback();
        }
      );

      onMounted(async () => {
        applyLanguage();
        state.status = t("loadingSession");
        await refreshAll();
      });

      return {
        activeSegmentBadge,
        activeSegmentSliderPercent,
        activeSegmentSliderValue,
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
        isBusy,
        isSessionDeleting,
        loadSession,
        loadVideoSegments,
        handlePlaybackEnded,
        nextClip,
        playbackSpeedOptions,
        previousClip,
        refreshAll,
        saveAutostartConfig,
        saveRecordingConfig,
        selectClipFromSlider,
        segmentSliderStyle,
        segmentRange,
        segmentTitle,
        selectSession,
        sessionGroups,
        sessionStateLabel,
        startRecording,
        state,
        stopTurboPlayback,
        stopRecording,
        syncTurboPlayback,
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
            <button id="control-refresh" class="ghost" type="button" @click="refreshAll" :disabled="state.isRefreshing || state.isStartingRecording || state.isStoppingRecording">{{ state.isRefreshing ? t('refreshing') : t('refresh') }}</button>
            <button id="control-start" class="primary" type="button" @click="startRecording" :disabled="state.isRefreshing || state.isStartingRecording || state.isStoppingRecording">{{ state.isStartingRecording ? t('startingRecording') : t('startRecording') }}</button>
            <button id="control-stop" type="button" @click="stopRecording" :disabled="state.isRefreshing || state.isStartingRecording || state.isStoppingRecording">{{ state.isStoppingRecording ? t('stoppingRecording') : t('stopRecording') }}</button>
          </div>
          <div
            id="control-feedback"
            class="feedback"
            v-if="state.controlFeedback"
            :data-variant="state.controlFeedbackVariant"
          >{{ state.controlFeedback }}</div>
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
            <button id="autostart-refresh" class="ghost" type="button" @click="refreshAll" :disabled="state.isRefreshing || state.isSavingAutostart">{{ state.isRefreshing ? t('refreshing') : t('refresh') }}</button>
            <button id="autostart-save" type="button" @click="saveAutostartConfig" :disabled="state.isSavingAutostart">{{ state.isSavingAutostart ? t('saving') : t('save') }}</button>
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
            <button id="recording-refresh" class="ghost" type="button" @click="refreshAll" :disabled="state.isRefreshing || state.isSavingRecordingSettings">{{ state.isRefreshing ? t('refreshing') : t('refresh') }}</button>
            <button id="recording-save" type="button" @click="saveRecordingConfig" :disabled="state.isSavingRecordingSettings">{{ state.isSavingRecordingSettings ? t('saving') : t('save') }}</button>
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
          <video id="video-player" ref="videoPlayer" controls playsinline muted @play="syncTurboPlayback" @pause="stopTurboPlayback" @ended="handlePlaybackEnded"></video>
          <div class="viewer-segment-slider" v-if="state.videoSegments.length > 0">
            <label class="field viewer-segment-slider-field" for="segment-slider">
              <span class="viewer-segment-slider-topline">
                <span id="segment-slider-label">{{ t('clipSelector') }}</span>
                <span class="viewer-segment-slider-value">{{ activeSegmentBadge }}</span>
              </span>
              <input
                id="segment-slider"
                class="segment-slider-input"
                type="range"
                min="1"
                :max="state.videoSegments.length"
                :value="activeSegmentSliderValue"
                :style="segmentSliderStyle"
                @input="selectClipFromSlider"
              />
              <span class="viewer-segment-slider-scale" aria-hidden="true">
                <span>1</span>
                <span>{{ state.videoSegments.length }}</span>
              </span>
            </label>
          </div>
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
                  <option v-for="speedOption in playbackSpeedOptions" :key="speedOption" :value="speedOption">
                    {{ speedOption }}x
                  </option>
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
              <button id="delete-day-sessions" class="ghost danger" type="button" @click="deleteDaySessions" :disabled="isBusy">{{ state.isDeletingDay ? t('deletingDay') : t('deleteDay') }}</button>
              <button id="delete-all-sessions" class="ghost danger" type="button" @click="deleteAllSessions" :disabled="isBusy">{{ state.isDeletingAll ? t('deletingAll') : t('deleteAll') }}</button>
              <button id="refresh-sessions" class="ghost" type="button" @click="refreshAll" :disabled="isBusy">{{ state.isRefreshing ? t('refreshing') : t('refresh') }}</button>
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
                  <button class="session-card-open" type="button" @click="selectSession(session.session_id)" :disabled="isBusy">
                    <div class="session-card-title">{{ formatClockTimeWithDate(session.started_at, state.language) }}</div>
                    <div class="session-card-subtitle">{{ sessionStateLabel(session) }}</div>
                    <div class="session-card-body">
                      <span class="session-card-duration">{{ state.loadingSessionId === session.session_id ? t('loadingSessionShort') : t('duration') + ' ' + formatElapsed((session.finished_at || session.last_activity_at) - session.started_at, state.language) }}</span>
                      <span class="session-card-size">{{ t('size') }} {{ formatBytes(session.total_bytes) }}</span>
                    </div>
                  </button>
                  <button class="session-card-delete ghost danger" type="button" @click="deleteSession(session.session_id)" :disabled="isBusy">{{ isSessionDeleting(session.session_id) ? t('deleting') : t('delete') }}</button>
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
