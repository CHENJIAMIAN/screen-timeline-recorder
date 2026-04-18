const state = {
  session: null,
  sessions: [],
  liveStatus: null,
  autostart: null,
  recordingSettings: null,
  activity: [],
  currentSessionId: currentSessionIdFromUrl(),
  languagePreference: currentLanguagePreferenceFromUrl() || loadStoredLanguagePreference(),
  language: "en",
  timestampMs: 0,
  overlayEnabled: false,
  statusPollTimer: null,
  playbackTimer: null,
  playbackRunning: false,
  playbackLoopEnabled: false,
  videoSegments: [],
  activeVideoSegmentIndex: -1,
  sessionFilter: "all",
  autostartFeedbackMessage: null,
  recordingSettingsFeedbackMessage: null,
  statusMessage: null,
};

const canvas = document.getElementById("canvas");
const ctx = canvas.getContext("2d");
const videoPlayer = document.getElementById("video-player");
const timestampInput = document.getElementById("timestamp");
const timestampFriendlyInput = document.getElementById("timestamp-friendly");
const advancedTimeToggle = document.getElementById("advanced-time-toggle");
const timelineInput = document.getElementById("timeline");
const timelineLabel = document.getElementById("timeline-label");
const activityStrip = document.getElementById("activity-strip");
const overlayToggle = document.getElementById("overlay");
const status = document.getElementById("status");
const sessionInfo = document.getElementById("session-info");
const recordingBadge = document.getElementById("recording-badge");
const statusSummary = document.getElementById("status-summary");
const sessionListGrid = document.getElementById("session-list-grid");
const sessionFilterSelect = document.getElementById("session-filter");
const deleteDaySessionsButton = document.getElementById("delete-day-sessions");
const deleteAllSessionsButton = document.getElementById("delete-all-sessions");
const refreshSessionsButton = document.getElementById("refresh-sessions");
const playbackToggleButton = document.getElementById("play-pause");
const playbackSpeedSelect = document.getElementById("playback-speed");
const playbackLoopToggle = document.getElementById("playback-loop");
const controlRefreshButton = document.getElementById("control-refresh");
const controlStartButton = document.getElementById("control-start");
const controlPauseButton = document.getElementById("control-pause");
const controlResumeButton = document.getElementById("control-resume");
const controlStopButton = document.getElementById("control-stop");
const languageSelect = document.getElementById("language-select");
const autostartState = document.getElementById("autostart-state");
const autostartEnabledInput = document.getElementById("autostart-enabled");
const autostartLoginInput = document.getElementById("autostart-login");
const autostartDelayInput = document.getElementById("autostart-delay");
const autostartOutputDirInput = document.getElementById("autostart-output-dir");
const autostartNote = document.getElementById("autostart-note");
const autostartFeedback = document.getElementById("autostart-feedback");
const autostartRefreshButton = document.getElementById("autostart-refresh");
const autostartSaveButton = document.getElementById("autostart-save");
const recordingSamplingIntervalInput = document.getElementById("recording-sampling-interval");
const recordingSensitivityModeInput = document.getElementById("recording-sensitivity-mode");
const recordingWorkingScaleInput = document.getElementById("recording-working-scale");
const recordingKeyframeIntervalInput = document.getElementById("recording-keyframe-interval");
const recordingBlockWidthInput = document.getElementById("recording-block-width");
const recordingBlockHeightInput = document.getElementById("recording-block-height");
const recordingBurnInEnabledInput = document.getElementById("recording-burn-in-enabled");
const recordingSettingsNote = document.getElementById("recording-settings-note");
const recordingSettingsFeedback = document.getElementById("recording-settings-feedback");
const recordingRefreshButton = document.getElementById("recording-refresh");
const recordingSaveButton = document.getElementById("recording-save");
const controlLogic = window.ScreenTimelineControlLogic;
const sessionListLogic = window.ScreenTimelineSessionListLogic;
const videoPlaybackLogic = window.ScreenTimelineVideoPlaybackLogic;

const I18N = {
  en: {
    appTitle: "Screen Timeline Viewer",
    sessionInfoLoading: "Loading session...",
    loadingSession: "Loading session metadata...",
    failedLoadSession: "Failed to load session ({status})",
    selectedTime: "Selected Time",
    timestampHelp: "Shows elapsed time since the session started and the wall-clock time. Turn on advanced input to type raw milliseconds.",
    advancedTime: "Advanced Time Input",
    timeline: "Timeline",
    load: "Load",
    prev: "-10s",
    next: "+10s",
    play: "Play",
    pausePlayback: "Pause",
    speed: "Speed",
    loopPlayback: "Loop playback",
    overlay: "Change overlay (legacy)",
    language: "Language",
    autostartTitle: "Autostart Recording",
    autostartSubtitle: "Automatically start recording after Windows login.",
    autostartEnabledLabel: "Enable autostart recording",
    autostartLoginLabel: "Start after login",
    autostartDelayLabel: "Delay before start (seconds)",
    autostartOutputLabel: "Recording output folder",
    autostartRefresh: "Refresh",
    autostartSave: "Save",
    autostartChecking: "Checking...",
    autostartEnabledState: "Enabled",
    autostartDisabledState: "Disabled",
    autostartUnsupportedState: "Unsupported",
    autostartLoading: "Loading autostart settings...",
    autostartSaving: "Saving autostart settings...",
    autostartSaved: "Autostart settings saved.",
    autostartRefreshed: "Autostart settings refreshed.",
    autostartFailed: "Failed to load autostart settings ({status})",
    autostartSaveFailed: "Failed to save autostart settings ({status})",
    autostartUnsupportedNote: "Autostart is only supported on Windows.",
    autostartNote: "Runs a Windows Scheduled Task to start recording after login.",
    recordingSettingsTitle: "Video Recording",
    recordingSettingsSubtitle: "Tune frame interval, capture scale, and time watermark for future video sessions.",
    recordingSamplingIntervalLabel: "Frame interval (ms)",
    recordingWorkingScaleLabel: "Capture scale",
    recordingBurnInEnabledLabel: "Burn in wall-clock time on recorded video",
    recordingRefresh: "Refresh",
    recordingSave: "Save",
    recordingLoading: "Loading recording settings...",
    recordingSaving: "Saving recording settings...",
    recordingSaved: "Recording settings saved. New sessions will use them.",
    recordingRefreshed: "Recording settings refreshed.",
    recordingFailed: "Failed to load recording settings ({status})",
    recordingSaveFailed: "Failed to save recording settings ({status})",
    recordingNote: "Frame interval controls video FPS. Capture scale controls resolution and file size. Time watermark is burned into the recorded video.",
    conservative: "Conservative",
    balanced: "Balanced",
    detailed: "Detailed",
    quickstartTitle: "How To Use",
    quickstartSubtitle: "Three quick steps to review a day.",
    quickstartStep1Title: "Pick a recent capture",
    quickstartStep1Body: "Click any Recent Sessions card to switch recordings.",
    quickstartStep2Title: "Move through time",
    quickstartStep2Body: "Drag the timeline or use the Prev/Next buttons.",
    quickstartStep3Title: "Press play",
    quickstartStep3Body: "Use Play and Speed to replay the timeline.",
    liveStatus: "Live Status",
    checking: "Checking...",
    refresh: "Refresh",
    startRecording: "Start New Recording",
    pause: "Pause",
    resume: "Resume",
    stop: "End Current Recording",
    deleteSession: "Delete",
    deleteSessionBusy: "Moving recording to recycle bin...",
    deleteSessionFailed: "Failed to delete session ({status})",
    deleteSessionActive: "Stop this recording before deleting it.",
    deleteSessionDone: "Recording moved to recycle bin. You can restore it there.",
    noSessionsAfterDelete: "Recording moved to recycle bin. You can restore it there.",
    startRecordingBusy: "Starting a new recording...",
    startRecordingFailed: "Failed to start a new recording ({status})",
    startRecordingConflict: "A recording is already active. Switch to the active session instead.",
    startRecordingStarted: "A new recording has started.",
    stopFinishedNotice: "This recording has ended. To keep recording, start a new session.",
    recentSessions: "Recent Sessions",
    recentSessionsSubtitle: "Sorted by most recent activity so the latest work stays on top",
    sessionFilter: "Date Filter",
    allDates: "All dates",
    deleteDaySessions: "Delete Day",
    deleteAllSessions: "Delete All",
    deleteDaySessionsBusy: "Moving this day's recordings to recycle bin...",
    deleteAllSessionsBusy: "Moving all recordings to recycle bin...",
    deleteDaySessionsDone: "That day's recordings were moved to recycle bin.",
    deleteAllSessionsDone: "All recordings were moved to recycle bin.",
    deleteDaySessionsFailed: "Failed to delete one or more recordings from this day.",
    deleteAllSessionsFailed: "Failed to delete one or more recordings.",
    sessionDayTitle: "{day}",
    sessionDaySummary: "{count} recordings | {size}",
    deleteDayGroup: "Delete This Day",
    ready: "Ready.",
    waitingHeartbeat: "Waiting for heartbeat...",
    heartbeatUnavailable: "Heartbeat unavailable",
    failedHeartbeat: "Failed to load heartbeat ({status})",
    heartbeatRefreshed: "Heartbeat refreshed.",
    loadingFrame: "Loading frame at {timestamp}...",
    failedFrame: "Failed to load frame ({status})",
    frameLoaded: "Frame loaded.",
    failedDecode: "Failed to decode frame.",
    failedPatches: "Failed to load patches ({status})",
    noSessions: "No sessions found",
    recordToPopulate: "Record a session to populate this list",
    failedLoadSessions: "Failed to load sessions",
    startedAt: "started {timestamp}",
    currentSessionSummary: "{timestamp} | {width}x{height}",
    sessionCardTitle: "{timestamp}",
    running: "Running",
    paused: "Paused",
    stopped: "Stopped",
    unknown: "Unknown",
    noHeartbeat: "No heartbeat",
    sendingCommand: "Sending {action} command...",
    controlFailed: "control failed ({status})",
    commandSent: "Session {action} command sent.",
    frames: "frames",
    identical: "identical",
    sampled: "sampled",
    diffs: "diffs",
    patchFrames: "patch frames",
    patchRegions: "patch regions",
    keyframes: "keyframes",
    duration: "duration",
    auto: "Auto",
    english: "English",
    chinese: "Chinese",
    sessionCardDuration: "Duration {duration}",
    sessionCardSize: "Size {size}",
    statusSummary:
      "Seen {frames} frames ({identical} repeats, {sampled} skipped samples). {diffRuns} diff runs produced {patchFrames} patch frames across {patchRegions} regions and {keyframes} keyframes over {duration}.",
    videoStatusSummary:
      "Video session at {width}x{height}. {segments} segments, {size}, duration {duration}.",
  },
  zh: {
    appTitle: "屏幕时间线查看器",
    sessionInfoLoading: "正在加载记录...",
    loadingSession: "正在加载记录信息...",
    failedLoadSession: "加载记录失败（{status}）",
    selectedTime: "当前时间",
    timestampHelp: "默认显示从开始到现在的经过时长和实际时钟时间。打开高级输入后，可以直接输入原始毫秒值。",
    advancedTime: "显示原始毫秒输入",
    timeline: "时间线",
    load: "加载",
    prev: "-10秒",
    next: "+10秒",
    play: "播放",
    pausePlayback: "暂停",
    speed: "速度",
    loopPlayback: "循环播放",
    overlay: "变化高亮（旧格式）",
    language: "语言",
    autostartTitle: "开机自动录制",
    autostartSubtitle: "在 Windows 登录后自动开始录制。",
    autostartEnabledLabel: "启用自动录制",
    autostartLoginLabel: "登录后启动",
    autostartDelayLabel: "延迟启动（秒）",
    autostartOutputLabel: "录制输出目录",
    autostartRefresh: "刷新",
    autostartSave: "保存",
    autostartChecking: "检查中...",
    autostartEnabledState: "已启用",
    autostartDisabledState: "未启用",
    autostartUnsupportedState: "当前不支持",
    autostartLoading: "正在加载自动录制设置...",
    autostartSaving: "正在保存自动录制设置...",
    autostartSaved: "自动录制设置已保存。",
    autostartRefreshed: "自动录制设置已刷新。",
    autostartFailed: "加载自动录制设置失败（{status}）",
    autostartSaveFailed: "保存自动录制设置失败（{status}）",
    autostartUnsupportedNote: "自动录制目前只支持 Windows。",
    autostartNote: "使用 Windows 计划任务在登录后开始录制。",
    recordingSettingsTitle: "视频录制",
    recordingSettingsSubtitle: "调整后续视频会话的帧间隔、采集比例和时间水印。",
    recordingSamplingIntervalLabel: "帧间隔（毫秒）",
    recordingWorkingScaleLabel: "采集比例",
    recordingBurnInEnabledLabel: "将实际时间烧录进录制视频",
    recordingRefresh: "刷新",
    recordingSave: "保存",
    recordingLoading: "正在加载录制设置...",
    recordingSaving: "正在保存录制设置...",
    recordingSaved: "录制设置已保存，新录制会使用这些参数。",
    recordingRefreshed: "录制设置已刷新。",
    recordingFailed: "加载录制设置失败（{status}）",
    recordingSaveFailed: "保存录制设置失败（{status}）",
    recordingNote: "帧间隔决定视频帧率，采集比例决定分辨率与文件大小，时间水印会直接烧录进视频画面。",
    conservative: "省资源",
    balanced: "平衡",
    detailed: "更敏感",
    quickstartTitle: "怎么用",
    quickstartSubtitle: "只要三步，就能回看这一天做了什么。",
    quickstartStep1Title: "先选一段最近记录",
    quickstartStep1Body: "点击“最近记录”里的任意卡片，就能切换到那段内容。",
    quickstartStep2Title: "再移动时间",
    quickstartStep2Body: "拖动时间线，或者点前后按钮，快速跳到想看的时刻。",
    quickstartStep3Title: "最后按播放",
    quickstartStep3Body: "用“播放”和“速度”把屏幕历史像轻量录屏一样回放出来。",
    liveStatus: "实时状态",
    checking: "检查中...",
    refresh: "刷新",
    startRecording: "开始新录制",
    pause: "暂停",
    resume: "继续",
    stop: "结束本次录制",
    deleteSession: "删除",
    deleteSessionBusy: "正在移到回收站...",
    deleteSessionFailed: "删除记录失败（{status}）",
    deleteSessionActive: "请先停止这条录制，再删除。",
    deleteSessionDone: "记录已移到回收站，可从回收站恢复。",
    noSessionsAfterDelete: "记录已移到回收站，可从回收站恢复。",
    startRecordingBusy: "正在开始新录制...",
    startRecordingFailed: "开始新录制失败（{status}）",
    startRecordingConflict: "已经有录制正在进行，请先切换到那条正在录制的会话。",
    startRecordingStarted: "新的录制已经开始。",
    stopFinishedNotice: "这次录制已结束。如需继续，请开始新录制。",
    recentSessions: "最近记录",
    recentSessionsSubtitle: "按最近活动排序，刚刚工作过的记录会排在前面",
    sessionFilter: "日期筛选",
    allDates: "全部日期",
    deleteDaySessions: "删除这一天",
    deleteAllSessions: "删除全部记录",
    deleteDaySessionsBusy: "正在将这一天的记录移到回收站...",
    deleteAllSessionsBusy: "正在将全部记录移到回收站...",
    deleteDaySessionsDone: "这一天的记录已移到回收站，可从回收站恢复。",
    deleteAllSessionsDone: "全部记录已移到回收站，可从回收站恢复。",
    deleteDaySessionsFailed: "删除这一天的部分记录失败。",
    deleteAllSessionsFailed: "删除全部记录时有部分失败。",
    sessionDayTitle: "{day}",
    sessionDaySummary: "{count} 条记录 | {size}",
    deleteDayGroup: "删除当天全部",
    ready: "就绪。",
    waitingHeartbeat: "正在等待状态心跳...",
    heartbeatUnavailable: "状态心跳不可用",
    failedHeartbeat: "加载状态失败（{status}）",
    heartbeatRefreshed: "状态已刷新。",
    loadingFrame: "正在加载 {timestamp} 的画面...",
    failedFrame: "加载画面失败（{status}）",
    frameLoaded: "画面已加载。",
    failedDecode: "画面解码失败。",
    failedPatches: "加载变化区域失败（{status}）",
    noSessions: "还没有可用记录",
    recordToPopulate: "先录一段屏幕，这里就会出现可切换的记录",
    failedLoadSessions: "加载记录列表失败",
    startedAt: "开始于 {timestamp}",
    currentSessionSummary: "{timestamp} | {width}x{height}",
    sessionCardTitle: "{timestamp}",
    running: "录制中",
    paused: "已暂停",
    stopped: "已停止",
    unknown: "未知",
    noHeartbeat: "没有状态",
    sendingCommand: "正在发送“{action}”命令...",
    controlFailed: "控制失败（{status}）",
    commandSent: "已发送“{action}”命令。",
    frames: "帧",
    identical: "重复",
    sampled: "抽样跳过",
    diffs: "差异检测",
    patchFrames: "变化帧",
    patchRegions: "变化区域",
    keyframes: "关键帧",
    duration: "时长",
    auto: "自动",
    english: "English",
    chinese: "中文",
    sessionCardDuration: "时长 {duration}",
    sessionCardSize: "大小 {size}",
    statusSummary:
      "这段记录已看到 {frames} 帧，其中 {identical} 帧与上一帧相同、{sampled} 次被快速跳过。系统写入了 {patchFrames} 个变化帧、{patchRegions} 个变化区域和 {keyframes} 个关键帧，当前累计时长 {duration}。",
    videoStatusSummary:
      "视频记录 {width}x{height}，共 {segments} 段，大小 {size}，时长 {duration}。",
  },
};

applyLanguage();
setStatusKey("ready");

document.getElementById("load").addEventListener("click", () => {
  stopPlayback();
  state.timestampMs = Number(timestampInput.value || state.timestampMs || 0);
  syncTimelineControls();
  loadFrame();
});

document.getElementById("prev").addEventListener("click", () => {
  stopPlayback();
  state.timestampMs = Math.max(sessionStartMs(), state.timestampMs - 10_000);
  syncTimelineControls();
  loadFrame();
});

document.getElementById("next").addEventListener("click", () => {
  stopPlayback();
  state.timestampMs = Math.min(sessionEndMs(), state.timestampMs + 10_000);
  syncTimelineControls();
  loadFrame();
});

advancedTimeToggle.addEventListener("change", () => {
  updateAdvancedTimeVisibility();
});

timelineInput.addEventListener("input", () => {
  stopPlayback();
  state.timestampMs = Number(timelineInput.value || 0);
  syncTimelineControls();
});

timelineInput.addEventListener("change", () => {
  stopPlayback();
  state.timestampMs = Number(timelineInput.value || 0);
  syncTimelineControls();
  loadFrame();
});

overlayToggle.addEventListener("change", () => {
  state.overlayEnabled = overlayToggle.checked;
  loadFrame();
});

refreshSessionsButton.addEventListener("click", async () => {
  await loadSessions();
});

sessionFilterSelect.addEventListener("change", () => {
  state.sessionFilter = sessionFilterSelect.value || "all";
  renderSessionList();
});

deleteDaySessionsButton.addEventListener("click", async () => {
  await deleteSessionsByScope("day");
});

deleteAllSessionsButton.addEventListener("click", async () => {
  await deleteSessionsByScope("all");
});

autostartRefreshButton.addEventListener("click", async () => {
  await loadAutostart();
});

autostartSaveButton.addEventListener("click", async () => {
  await saveAutostart();
});

recordingRefreshButton.addEventListener("click", async () => {
  await loadRecordingSettings();
});

recordingSaveButton.addEventListener("click", async () => {
  await saveRecordingSettings();
});

languageSelect.addEventListener("change", () => {
  state.languagePreference = languageSelect.value || "auto";
  window.localStorage.setItem("viewerLanguage", state.languagePreference);
  updateUrlLanguage(state.languagePreference);
  applyLanguage();
  renderSessionList();
  renderLiveStatus();
  syncTimelineControls();
});

controlRefreshButton.addEventListener("click", async () => {
  await refreshLiveState();
});

controlStartButton.addEventListener("click", async () => {
  await startNewRecording();
});

controlPauseButton.addEventListener("click", async () => {
  await sendControlAction("pause");
});

controlResumeButton.addEventListener("click", async () => {
  await sendControlAction("resume");
});

controlStopButton.addEventListener("click", async () => {
  await sendControlAction("stop");
});

playbackToggleButton.addEventListener("click", () => {
  togglePlayback();
});

playbackLoopToggle.addEventListener("change", () => {
  state.playbackLoopEnabled = playbackLoopToggle.checked;
});

playbackSpeedSelect.addEventListener("change", () => {
  if (isVideoSession()) {
    videoPlayer.playbackRate = Number(playbackSpeedSelect.value || 1);
  }
});

videoPlayer.addEventListener("play", () => {
  if (!isVideoSession()) {
    return;
  }
  state.playbackRunning = true;
  playbackToggleButton.textContent = t("pausePlayback");
});

videoPlayer.addEventListener("pause", () => {
  if (!isVideoSession()) {
    return;
  }
  state.playbackRunning = false;
  playbackToggleButton.textContent = t("play");
});

videoPlayer.addEventListener("timeupdate", () => {
  if (!isVideoSession() || state.activeVideoSegmentIndex < 0) {
    return;
  }

  const segment = state.videoSegments[state.activeVideoSegmentIndex];
  if (!segment) {
    return;
  }

  const absoluteTimestamp =
    Number(segment.started_at || 0) + Math.round(Number(videoPlayer.currentTime || 0) * 1000);
  state.timestampMs = Math.min(sessionEndMs(), Math.max(sessionStartMs(), absoluteTimestamp));
  syncTimelineControls();
});

videoPlayer.addEventListener("ended", async () => {
  if (!isVideoSession()) {
    return;
  }

  const nextIndex = state.activeVideoSegmentIndex + 1;
  if (nextIndex < state.videoSegments.length) {
    state.timestampMs = Number(state.videoSegments[nextIndex].started_at || sessionStartMs());
    syncTimelineControls();
    await loadVideoFrame({ autoplay: true, forceSeek: true, preserveStatus: true });
    return;
  }

  if (state.playbackLoopEnabled) {
    state.timestampMs = sessionStartMs();
    syncTimelineControls();
    await loadVideoFrame({ autoplay: true, forceSeek: true, preserveStatus: true });
    return;
  }

  stopPlayback();
});

async function loadSession() {
  stopPlayback();
  applyLanguage();
  setStatusKey("loadingSession");
  const response = await fetch(apiUrl("/api/session"));
  if (!response.ok) {
    setStatusKey("failedLoadSession", { status: response.status });
    return;
  }

  state.session = await response.json();
  state.currentSessionId = state.session.session_id;
  state.timestampMs = state.session.started_at || 0;
  state.overlayEnabled = Boolean(state.session.viewer_overlay_enabled_by_default);
  overlayToggle.checked = state.overlayEnabled;
  applyLanguage();
  configureTimeline();
  syncTimelineControls();

  canvas.width = state.session.working_width;
  canvas.height = state.session.working_height;
  syncPlaybackSurface();
  sessionInfo.textContent = t("currentSessionSummary", {
    timestamp: formatClockTimeWithDate(state.session.started_at),
    width: state.session.working_width,
    height: state.session.working_height,
  });
  sessionInfo.textContent = `${sessionInfo.textContent} | ${formatRecordingFormatLabel(state.session)}`;

  await loadSessions();
  await loadStatus();
  await loadAutostart();
  await loadRecordingSettings();
  await loadActivity();
  if (isVideoSession()) {
    await loadVideoSegments();
  } else {
    state.videoSegments = [];
    state.activeVideoSegmentIndex = -1;
  }
  startStatusPolling();
  await loadFrame({ preserveStatus: true });
}

async function loadVideoSegments() {
  const response = await fetch(apiUrl("/api/segments"));
  if (!response.ok) {
    state.videoSegments = [];
    renderLiveStatus();
    return;
  }
  state.videoSegments = await response.json();
  renderLiveStatus();
}

async function loadAutostart() {
  setAutostartFeedback("working", "autostartLoading");
  setStatusKey("autostartLoading");
  const response = await fetch(rootApiUrl("/api/autostart"));
  if (!response.ok) {
    setAutostartFeedback("error", "autostartFailed", { status: response.status });
    setStatusKey("autostartFailed", { status: response.status });
    return;
  }

  state.autostart = await response.json();
  renderAutostart();
  setAutostartFeedback("success", "autostartRefreshed");
}

async function saveAutostart() {
  const params = new URLSearchParams({
    enabled: String(autostartEnabledInput.checked),
    start_on_login: "true",
    delay_seconds: String(Number(autostartDelayInput.value || 0)),
    output_dir: autostartOutputDirInput.value || "",
  });

  setAutostartBusy(true);
  setAutostartFeedback("working", "autostartSaving");
  setStatusKey("autostartSaving");
  const response = await fetch(rootApiUrl(`/api/autostart/save?${params.toString()}`));
  if (!response.ok) {
    setAutostartBusy(false);
    setAutostartFeedback("error", "autostartSaveFailed", { status: response.status });
    setStatusKey("autostartSaveFailed", { status: response.status });
    return;
  }

  state.autostart = await response.json();
  renderAutostart();
  setAutostartBusy(false);
  setAutostartFeedback("success", "autostartSaved");
  setStatusKey("autostartSaved");
}

async function loadRecordingSettings() {
  setRecordingSettingsFeedback("working", "recordingLoading");
  const response = await fetch(rootApiUrl("/api/recording-settings"));
  if (!response.ok) {
    setRecordingSettingsFeedback("error", "recordingFailed", { status: response.status });
    setStatusKey("recordingFailed", { status: response.status });
    return;
  }

  state.recordingSettings = await response.json();
  renderRecordingSettings();
  setRecordingSettingsFeedback("success", "recordingRefreshed");
}

async function saveRecordingSettings() {
  const params = new URLSearchParams({
    sampling_interval_ms: String(Number(recordingSamplingIntervalInput.value || 0)),
    working_scale: String(Number(recordingWorkingScaleInput.value || 0)),
    burn_in_enabled: String(recordingBurnInEnabledInput.checked),
  });

  setRecordingSettingsBusy(true);
  setRecordingSettingsFeedback("working", "recordingSaving");
  setStatusKey("recordingSaving");
  const response = await fetch(rootApiUrl(`/api/recording-settings/save?${params.toString()}`));
  if (!response.ok) {
    setRecordingSettingsBusy(false);
    setRecordingSettingsFeedback("error", "recordingSaveFailed", { status: response.status });
    setStatusKey("recordingSaveFailed", { status: response.status });
    return;
  }

  state.recordingSettings = await response.json();
  renderRecordingSettings();
  setRecordingSettingsBusy(false);
  setRecordingSettingsFeedback("success", "recordingSaved");
  setStatusKey("recordingSaved");
}

async function loadSessions() {
  const response = await fetch("/api/sessions");
  if (!response.ok) {
    sessionListGrid.innerHTML = `<article class="session-card placeholder"><div class="session-card-title">${escapeHtml(t("failedLoadSessions"))}</div><div class="session-card-subtitle">HTTP ${response.status}</div></article>`;
    return;
  }

  state.sessions = await response.json();
  renderSessionList();
}

async function deleteSessionById(sessionId) {
  return deleteSessionByIdInternal(sessionId, false);
}

async function deleteSessionByIdInternal(sessionId, silent) {
  if (!silent) {
    setStatusKey("deleteSessionBusy");
  }
  const response = await fetch(rootApiUrl(`/api/control?action=delete&session_id=${encodeURIComponent(sessionId)}`));
  const payload = await response.json().catch(() => null);
  if (!response.ok || !payload || payload.ok !== true) {
    if (!silent) {
      if (payload && payload.error && payload.error.includes("active session")) {
        setStatusKey("deleteSessionActive");
      } else if (payload && payload.error) {
        setRawStatus(payload.error);
      } else {
        setStatusKey("deleteSessionFailed", { status: response.status });
      }
    }
    return false;
  }

  if (!silent && state.currentSessionId === sessionId) {
    if (state.sessions.length > 0) {
      await switchToSession(state.sessions[0].session_id);
    } else {
      state.currentSessionId = null;
      state.session = null;
      state.liveStatus = null;
      state.activity = [];
      ctx.clearRect(0, 0, canvas.width, canvas.height);
      sessionInfo.textContent = t("sessionInfoLoading");
      renderSessionList();
      renderLiveStatus();
      renderActivityStrip();
      if (!silent) {
        setStatusKey("noSessionsAfterDelete");
      }
      return true;
    }
  }
  if (!silent) {
    setStatusKey("deleteSessionDone");
  }
  return true;
}

async function deleteSessionsByScope(scope) {
  const targetSessions =
    scope === "all" ? [...state.sessions] : filteredSessions();
  if (!targetSessions.length) {
    return;
  }

  setStatusKey(scope === "all" ? "deleteAllSessionsBusy" : "deleteDaySessionsBusy");
  setBulkDeleteButtonsBusy(true);

  let allSucceeded = true;
  for (const session of targetSessions) {
    const deleted = await deleteSessionByIdInternal(session.session_id, true);
    if (!deleted) {
      allSucceeded = false;
    }
  }

  await loadSessions();
  await reconcileCurrentSessionAfterDeletion();
  setBulkDeleteButtonsBusy(false);
  if (allSucceeded) {
    setStatusKey(scope === "all" ? "deleteAllSessionsDone" : "deleteDaySessionsDone");
  } else {
    setStatusKey(scope === "all" ? "deleteAllSessionsFailed" : "deleteDaySessionsFailed");
  }
}

async function startNewRecording() {
  setStatusKey("startRecordingBusy");
  setControlButtonsDisabled(true);
  const response = await fetch(rootApiUrl("/api/control?action=start"));
  const payload = await response.json().catch(() => null);
  if (!response.ok || !payload || payload.ok !== true) {
    if (payload && payload.error && payload.error.includes("already active")) {
      setStatusKey("startRecordingConflict");
    } else if (payload && payload.error) {
      setRawStatus(payload.error);
    } else {
      setStatusKey("startRecordingFailed", { status: response.status });
    }
    await refreshLiveState();
    return;
  }

  if (payload.session_id) {
    await waitForSessionAndSwitch(payload.session_id);
  } else {
    await refreshLiveState();
  }
  setStatusKey("startRecordingStarted");
}

async function loadStatus() {
  const response = await fetch(apiUrl("/api/status"));
  if (!response.ok) {
    state.liveStatus = null;
    recordingBadge.textContent = t("heartbeatUnavailable");
    recordingBadge.dataset.state = "unknown";
    statusSummary.textContent = t("failedHeartbeat", { status: response.status });
    syncControlButtons("unknown");
    return;
  }

  state.liveStatus = await response.json();
  renderLiveStatus();
  configureTimeline();
  syncTimelineControls();
  renderSessionList();
}

async function refreshLiveState() {
  await loadStatus();
  await loadSessions();
  setStatusKey("heartbeatRefreshed");
}

async function loadActivity() {
  const response = await fetch(apiUrl("/api/activity"));
  if (!response.ok) {
    state.activity = [];
    renderActivityStrip();
    return;
  }

  state.activity = await response.json();
  renderActivityStrip();
}

async function loadFrame(options = {}) {
  if (!state.session) return;
  if (isVideoSession()) {
    await loadVideoFrame(options);
    return;
  }
  const preserveStatus = Boolean(options.preserveStatus);
  const silent = state.playbackRunning;
  if (!silent && !preserveStatus) {
    setStatusKey("loadingFrame", { timestamp: formatTimelineLabel(state.timestampMs) });
  }
  const response = await fetch(apiUrl(`/api/frame?ts=${state.timestampMs}`));
  if (!response.ok) {
    if (!preserveStatus) {
      setStatusKey("failedFrame", { status: response.status });
    }
    return;
  }
  const blob = await response.blob();
  const imageUrl = URL.createObjectURL(blob);
  const img = new Image();
  img.onload = async () => {
    ctx.clearRect(0, 0, canvas.width, canvas.height);
    ctx.drawImage(img, 0, 0);
    URL.revokeObjectURL(imageUrl);
    if (state.overlayEnabled) {
      await drawOverlay();
    }
    if (!silent && !preserveStatus) {
      setStatusKey("frameLoaded");
    }
  };
  img.onerror = () => {
    if (!preserveStatus) {
      setStatusKey("failedDecode");
    }
    URL.revokeObjectURL(imageUrl);
  };
  img.src = imageUrl;
}

async function loadVideoFrame(options = {}) {
  if (!videoPlaybackLogic) {
    return;
  }
  const preserveStatus = Boolean(options.preserveStatus);
  const autoplay = options.autoplay ?? state.playbackRunning;
  const silent = Boolean(autoplay);
  if (!silent && !preserveStatus) {
    setStatusKey("loadingFrame", { timestamp: formatTimelineLabel(state.timestampMs) });
  }

  const segmentIndex = findVideoSegmentIndex(state.timestampMs);
  if (segmentIndex < 0) {
    if (!preserveStatus) {
      setStatusKey("failedFrame", { status: 404 });
    }
    return;
  }

  const segment = state.videoSegments[segmentIndex];
  const targetTime = videoPlaybackLogic.getVideoTargetTimeSeconds(segment, state.timestampMs);
  const source = apiUrl(`/${segment.relative_path}`);
  const needsSourceSwap =
    state.activeVideoSegmentIndex !== segmentIndex || !videoPlayer.src.endsWith(source);

  if (needsSourceSwap) {
    state.activeVideoSegmentIndex = segmentIndex;
    videoPlayer.src = source;
    await new Promise((resolve, reject) => {
      const onLoaded = () => {
        videoPlayer.removeEventListener("loadedmetadata", onLoaded);
        videoPlayer.removeEventListener("error", onError);
        resolve();
      };
      const onError = () => {
        videoPlayer.removeEventListener("loadedmetadata", onLoaded);
        videoPlayer.removeEventListener("error", onError);
        reject(new Error("video load failed"));
      };
      videoPlayer.addEventListener("loadedmetadata", onLoaded);
      videoPlayer.addEventListener("error", onError);
      videoPlayer.load();
    }).catch(() => null);
  }

  if (
    Number.isFinite(targetTime) &&
    (options.forceSeek ||
      needsSourceSwap ||
      videoPlaybackLogic.shouldSeekVideo(videoPlayer.currentTime, targetTime))
  ) {
    try {
      videoPlayer.currentTime = targetTime;
    } catch (_) {}
  }
  videoPlayer.playbackRate = Number(playbackSpeedSelect.value || 1);
  if (autoplay) {
    await videoPlayer.play().catch(() => null);
  } else {
    videoPlayer.pause();
  }

  if (!silent && !preserveStatus) {
    setStatusKey("frameLoaded");
  }
}

async function drawOverlay() {
  const response = await fetch(apiUrl(`/api/patches?ts=${state.timestampMs}`));
  if (!response.ok) {
    setStatusKey("failedPatches", { status: response.status });
    return;
  }
  const patches = await response.json();
  if (!patches.length) {
    return;
  }
  const latestTimestamp = patches.reduce(
    (maxTimestamp, patch) => Math.max(maxTimestamp, Number(patch.timestamp_ms) || 0),
    0
  );
  const visiblePatches = patches.filter(
    (patch) => Number(patch.timestamp_ms) === latestTimestamp
  );
  ctx.save();
  ctx.strokeStyle = "rgba(255, 145, 0, 0.28)";
  ctx.lineWidth = 0.5;
  for (const patch of visiblePatches) {
    ctx.strokeRect(patch.x + 0.25, patch.y + 0.25, Math.max(0, patch.width - 0.5), Math.max(0, patch.height - 0.5));
  }
  ctx.restore();
}

function renderSessionList() {
  const sessions = filteredSessions();
  renderSessionFilterOptions();
  updateBulkDeleteButtons();

  if (!sessions.length) {
    const emptySubtitle = state.sessions.length
      ? t("allDates")
      : t("recordToPopulate");
    sessionListGrid.innerHTML = `<article class="session-card placeholder"><div class="session-card-title">${escapeHtml(t("noSessions"))}</div><div class="session-card-subtitle">${escapeHtml(emptySubtitle)}</div></article>`;
    return;
  }

  const groups = sessionListLogic.groupSessionsByDay(sessions);
  sessionListGrid.innerHTML = groups
    .map((group) => {
      const cards = group.sessions
        .map((session) => {
          const isCurrent = session.session_id === state.currentSessionId;
          const liveState = sessionListState(session);
          const formatLabel = formatRecordingFormatLabel(session);
          const formatKey = session.recording_format === "video-segments" ? "video" : "legacy";
          return `
        <article class="session-card${isCurrent ? " current" : ""}">
          <button class="session-card-open" data-session-id="${escapeHtml(session.session_id)}" title="${escapeHtml(session.session_id)}" type="button">
            <span class="session-card-title">${escapeHtml(formatSessionCardTitle(session.started_at))}</span>
            <span class="session-card-subtitle">${escapeHtml(formatSessionCardSubtitle(session))}</span>
            <span class="session-card-body">
            <span class="session-format-badge" data-format="${formatKey}">${escapeHtml(formatLabel)}</span>
            <span class="session-card-duration">${escapeHtml(
              t("sessionCardDuration", { duration: formatDuration(session.finished_at ?? session.last_activity_at, session.started_at) })
            )}</span>
            <span class="session-card-size">${escapeHtml(
              t("sessionCardSize", { size: formatBytes(session.total_bytes || 0) })
            )}</span>
            <span class="session-card-time" data-state="${liveState}">${formatStatusLabel(liveState)}</span>
            </span>
          </button>
          <button class="ghost session-card-delete" data-delete-session-id="${escapeHtml(session.session_id)}" type="button">${escapeHtml(t("deleteSession"))}</button>
        </article>
      `;
        })
        .join("");

      return `
        <section class="session-day-group">
          <div class="session-day-header">
            <div>
              <div class="session-day-title">${escapeHtml(t("sessionDayTitle", { day: formatDayLabel(group.dayKey) }))}</div>
              <div class="session-day-summary">${escapeHtml(
                t("sessionDaySummary", { count: group.sessionCount, size: formatBytes(group.totalBytes) })
              )}</div>
            </div>
            <button class="ghost danger session-day-delete" data-delete-day="${escapeHtml(group.dayKey)}" type="button">${escapeHtml(
              t("deleteDayGroup")
            )}</button>
          </div>
          <div class="session-day-grid">${cards}</div>
        </section>
      `;
    })
    .join("");

  for (const button of sessionListGrid.querySelectorAll("[data-session-id]")) {
    button.addEventListener("click", async () => {
      const nextSessionId = button.getAttribute("data-session-id");
      if (!nextSessionId || nextSessionId === state.currentSessionId) {
        return;
      }

      await switchToSession(nextSessionId);
    });
  }

  for (const button of sessionListGrid.querySelectorAll("[data-delete-session-id]")) {
    button.addEventListener("click", async () => {
      const sessionId = button.getAttribute("data-delete-session-id");
      if (!sessionId) {
        return;
      }

      await deleteSessionById(sessionId);
    });
  }

  for (const button of sessionListGrid.querySelectorAll("[data-delete-day]")) {
    button.addEventListener("click", async () => {
      const dayKey = button.getAttribute("data-delete-day");
      if (!dayKey) {
        return;
      }

      state.sessionFilter = dayKey;
      sessionFilterSelect.value = dayKey;
      await deleteSessionsByScope("day");
    });
  }
}

function startStatusPolling() {
  stopStatusPolling();
  state.statusPollTimer = window.setInterval(async () => {
    await loadStatus();
    await loadSessions();
    if (state.liveStatus && normalizeStatusState(state.liveStatus) === "stopped") {
      stopStatusPolling();
    }
  }, 2000);
}

async function sendControlAction(action) {
  const controlState = currentControlState();
  if (!controlState.controlSessionId) {
    setRawStatus("No controllable session is available.");
    return;
  }

  setStatusKey("sendingCommand", { action: formatStatusAction(action) });
  setControlButtonsDisabled(true);
  const response = await fetch(
    rootApiUrl(
      `/api/control?action=${encodeURIComponent(action)}&session_id=${encodeURIComponent(controlState.controlSessionId)}`
    )
  );
  const payload = await response.json().catch(() => null);
  if (!response.ok || !payload || payload.ok !== true) {
    const message =
      payload && payload.error ? payload.error : t("controlFailed", { status: response.status });
    setRawStatus(message);
    await refreshLiveState();
    return;
  }

  if (payload.status) {
    state.liveStatus = payload.status;
    renderLiveStatus();
  } else {
    await loadStatus();
  }
  await loadSessions();
  if (action === "stop") {
    stopStatusPolling();
    setStatusKey("stopFinishedNotice");
  } else {
    setStatusKey("commandSent", { action: formatStatusAction(action) });
  }
}

function stopStatusPolling() {
  if (state.statusPollTimer !== null) {
    window.clearInterval(state.statusPollTimer);
    state.statusPollTimer = null;
  }
}

function togglePlayback() {
  if (state.playbackRunning) {
    stopPlayback();
    return;
  }

  startPlayback();
}

function startPlayback() {
  stopPlayback();
  if (isVideoSession()) {
    state.playbackRunning = true;
    playbackToggleButton.textContent = t("pausePlayback");
    void loadVideoFrame({ autoplay: true, preserveStatus: true });
    return;
  }
  state.playbackRunning = true;
  playbackToggleButton.textContent = t("pausePlayback");
  const stepMs = playbackStepMs();
  state.playbackTimer = window.setInterval(async () => {
    const endMs = sessionEndMs();
    const startMs = sessionStartMs();
    const reachedEnd = state.timestampMs >= endMs;
    if (reachedEnd && state.playbackLoopEnabled) {
      state.timestampMs = startMs;
      syncTimelineControls();
      await loadFrame();
      return;
    }

    const nextTimestamp = Math.min(endMs, state.timestampMs + stepMs);
    state.timestampMs = nextTimestamp;
    syncTimelineControls();
    await loadFrame();
    if (state.timestampMs >= endMs && !state.playbackLoopEnabled) {
      stopPlayback();
    }
  }, stepMs);
}

function stopPlayback() {
  if (state.playbackTimer !== null) {
    window.clearInterval(state.playbackTimer);
    state.playbackTimer = null;
  }
  if (isVideoSession()) {
    state.playbackRunning = false;
    videoPlayer.pause();
    playbackToggleButton.textContent = t("play");
    return;
  }
  state.playbackRunning = false;
  playbackToggleButton.textContent = t("play");
}

function playbackStepMs() {
  const speed = Number(playbackSpeedSelect.value || 1);
  return Math.max(100, Math.round(1000 / speed));
}

function renderLiveStatus() {
  if (!state.liveStatus) {
    recordingBadge.textContent = t("noHeartbeat");
    recordingBadge.dataset.state = "unknown";
    statusSummary.textContent = t("waitingHeartbeat");
    syncControlButtons("unknown");
    return;
  }

  const { stats } = state.liveStatus;
  const liveState = normalizeStatusState(state.liveStatus);
  recordingBadge.textContent = formatStatusLabel(liveState);
  recordingBadge.dataset.state = liveState;
  if (isVideoSession()) {
    const currentSession = state.sessions.find((session) => session.session_id === state.currentSessionId);
    statusSummary.textContent = t("videoStatusSummary", {
      width: state.session ? state.session.working_width : 0,
      height: state.session ? state.session.working_height : 0,
      segments: state.videoSegments.length,
      size: formatBytes(currentSession ? currentSession.total_bytes || 0 : 0),
      duration: formatElapsed(Math.max(0, stats.finished_at - stats.started_at)),
    });
    syncControlButtons(liveState);
    return;
  }
  statusSummary.textContent = t("statusSummary", {
    frames: stats.frames_seen,
    identical: stats.identical_frames_skipped,
    sampled: stats.sampled_precheck_skipped,
    diffRuns: stats.diff_runs,
    patchFrames: stats.patch_frames_written,
    patchRegions: stats.patch_regions_written,
    keyframes: stats.keyframes_written,
    duration: formatElapsed(Math.max(0, stats.finished_at - stats.started_at)),
  });
  syncControlButtons(liveState);
}

function renderAutostart() {
  if (!state.autostart) {
    autostartState.textContent = t("autostartChecking");
    autostartNote.textContent = t("autostartNote");
    return;
  }

  const { supported, settings } = state.autostart;
  autostartEnabledInput.checked = Boolean(settings.enabled);
  autostartLoginInput.checked = true;
  autostartDelayInput.value = String(settings.delay_seconds ?? 0);
  autostartOutputDirInput.value = settings.output_dir || "";
  autostartLoginInput.disabled = true;
  autostartEnabledInput.disabled = !supported;
  autostartDelayInput.disabled = !supported;
  autostartOutputDirInput.disabled = !supported;
  autostartSaveButton.disabled = !supported;

  if (!supported) {
    autostartState.textContent = t("autostartUnsupportedState");
    autostartNote.textContent = t("autostartUnsupportedNote");
    return;
  }

  autostartState.textContent = settings.enabled
    ? t("autostartEnabledState")
    : t("autostartDisabledState");
  autostartNote.textContent = t("autostartNote");
}

function setAutostartBusy(busy) {
  const supported = !state.autostart || state.autostart.supported;
  autostartSaveButton.disabled = busy || !supported;
  autostartRefreshButton.disabled = busy;
  autostartSaveButton.textContent = busy ? t("autostartSaving") : t("autostartSave");
}

function setAutostartFeedback(variant, messageKey, vars = {}) {
  state.autostartFeedbackMessage = { variant, key: messageKey, vars };
  renderAutostartFeedback();
}

function renderAutostartFeedback() {
  const message = state.autostartFeedbackMessage;
  if (!message) {
    autostartFeedback.classList.add("hidden");
    return;
  }

  autostartFeedback.classList.remove("hidden");
  autostartFeedback.dataset.variant = message.variant;
  autostartFeedback.textContent = t(message.key, message.vars);
}

function renderRecordingSettings() {
  if (!state.recordingSettings) {
    return;
  }

  recordingSamplingIntervalInput.value = String(state.recordingSettings.sampling_interval_ms ?? 500);
  recordingWorkingScaleInput.value = String(state.recordingSettings.working_scale ?? 0.5);
  recordingBurnInEnabledInput.checked = Boolean(state.recordingSettings.burn_in_enabled ?? true);
}

function setRecordingSettingsBusy(busy) {
  recordingSaveButton.disabled = busy;
  recordingRefreshButton.disabled = busy;
  recordingSaveButton.textContent = busy ? t("recordingSaving") : t("recordingSave");
}

function setRecordingSettingsFeedback(variant, messageKey, vars = {}) {
  state.recordingSettingsFeedbackMessage = { variant, key: messageKey, vars };
  renderRecordingSettingsFeedback();
}

function renderRecordingSettingsFeedback() {
  const message = state.recordingSettingsFeedbackMessage;
  if (!message) {
    recordingSettingsFeedback.classList.add("hidden");
    return;
  }

  recordingSettingsFeedback.classList.remove("hidden");
  recordingSettingsFeedback.dataset.variant = message.variant;
  recordingSettingsFeedback.textContent = t(message.key, message.vars);
}

function configureTimeline() {
  timelineInput.min = String(sessionStartMs());
  timelineInput.max = String(sessionEndMs());
  timelineInput.step = "1";
}

function syncTimelineControls() {
  const clamped = Math.min(sessionEndMs(), Math.max(sessionStartMs(), state.timestampMs));
  state.timestampMs = clamped;
  timestampInput.value = String(clamped);
  timestampFriendlyInput.value = formatTimelineLabel(clamped);
  timelineInput.value = String(clamped);
  timelineLabel.textContent = formatTimelineLabel(clamped);
  renderActivityStrip();
}

function renderActivityStrip() {
  if (!state.activity.length || sessionEndMs() <= sessionStartMs()) {
    activityStrip.innerHTML = "";
    return;
  }

  const start = sessionStartMs();
  const range = Math.max(1, sessionEndMs() - start);
  activityStrip.innerHTML = state.activity
    .map((point) => {
      const leftPercent = ((point.timestamp_ms - start) / range) * 100;
      const isCurrent = Math.abs(point.timestamp_ms - state.timestampMs) <= 500;
      return `<span class="activity-marker${isCurrent ? " current" : ""}" style="left:${leftPercent}%"></span>`;
    })
    .join("");
}

function isVideoSession() {
  return state.session && state.session.recording_format === "video-segments";
}

function syncPlaybackSurface() {
  const showVideo = isVideoSession();
  videoPlayer.classList.toggle("hidden", !showVideo);
  canvas.classList.toggle("hidden", showVideo);
  overlayToggle.disabled = showVideo;
}

function findVideoSegmentIndex(timestampMs) {
  if (!videoPlaybackLogic) {
    return -1;
  }
  return videoPlaybackLogic.findVideoSegmentIndex(
    state.videoSegments,
    timestampMs,
    sessionEndMs()
  );
}

function sessionStartMs() {
  return state.session ? Number(state.session.started_at || 0) : 0;
}

function sessionEndMs() {
  if (!state.session) {
    return 0;
  }

  const sessionFinished = state.session.finished_at;
  if (sessionFinished !== null && sessionFinished !== undefined) {
    return Number(sessionFinished);
  }

  if (state.liveStatus && state.liveStatus.stats) {
    return Number(state.liveStatus.stats.finished_at || state.session.started_at || 0);
  }

  return Number(state.session.started_at || 0);
}

function currentSessionIdFromUrl() {
  const params = new URLSearchParams(window.location.search);
  return params.get("session_id");
}

function updateUrlSessionId(sessionId) {
  const url = new URL(window.location.href);
  url.searchParams.set("session_id", sessionId);
  window.history.replaceState({}, "", url);
}

function apiUrl(path) {
  const url = new URL(path, window.location.origin);
  if (state.currentSessionId) {
    url.searchParams.set("session_id", state.currentSessionId);
  }
  return url.toString();
}

function rootApiUrl(path) {
  return new URL(path, window.location.origin).toString();
}

function formatDuration(finishedAt, startedAt) {
  const durationMs = Math.max(0, (finishedAt ?? startedAt) - startedAt);
  return formatElapsed(durationMs);
}

function formatBytes(bytes) {
  const value = Number(bytes || 0);
  if (value < 1024) {
    return `${value} B`;
  }
  if (value < 1024 * 1024) {
    return `${(value / 1024).toFixed(1)} KB`;
  }
  if (value < 1024 * 1024 * 1024) {
    return `${(value / (1024 * 1024)).toFixed(1)} MB`;
  }
  return `${(value / (1024 * 1024 * 1024)).toFixed(1)} GB`;
}

function filteredSessions() {
  return sessionListLogic.filterSessionsByDay(state.sessions, state.sessionFilter);
}

function renderSessionFilterOptions() {
  const availableDays = sessionListLogic.listAvailableSessionDays(state.sessions);
  const nextValue =
    state.sessionFilter !== "all" && !availableDays.includes(state.sessionFilter)
      ? "all"
      : state.sessionFilter;
  state.sessionFilter = nextValue;

  sessionFilterSelect.innerHTML = [
    `<option value="all">${escapeHtml(t("allDates"))}</option>`,
    ...availableDays.map((dayKey) => `<option value="${escapeHtml(dayKey)}">${escapeHtml(formatDayLabel(dayKey))}</option>`),
  ].join("");
  sessionFilterSelect.value = state.sessionFilter;
}

function updateBulkDeleteButtons() {
  deleteAllSessionsButton.disabled = state.sessions.length === 0;
  deleteDaySessionsButton.disabled = filteredSessions().length === 0;
}

function setBulkDeleteButtonsBusy(busy) {
  deleteAllSessionsButton.disabled = busy || state.sessions.length === 0;
  deleteDaySessionsButton.disabled = busy || filteredSessions().length === 0;
  refreshSessionsButton.disabled = busy;
}

function formatDayLabel(dayKey) {
  const [year, month, day] = dayKey.split("-");
  if (!year || !month || !day) {
    return dayKey;
  }
  return state.language === "zh" ? `${year}年${month}月${day}日` : `${year}-${month}-${day}`;
}

async function reconcileCurrentSessionAfterDeletion() {
  if (!state.currentSessionId) {
    return;
  }

  const stillExists = state.sessions.some((session) => session.session_id === state.currentSessionId);
  if (stillExists) {
    return;
  }

  if (state.sessions.length > 0) {
    await switchToSession(state.sessions[0].session_id);
    return;
  }

  state.currentSessionId = null;
  state.session = null;
  state.liveStatus = null;
  state.activity = [];
  ctx.clearRect(0, 0, canvas.width, canvas.height);
  sessionInfo.textContent = t("sessionInfoLoading");
  renderSessionList();
  renderLiveStatus();
  renderActivityStrip();
}

function formatSessionStart(timestampMs) {
  return t("startedAt", { timestamp: formatClockTime(timestampMs) });
}

function formatSessionCardTitle(timestampMs) {
  return t("sessionCardTitle", { timestamp: formatClockTimeWithDate(timestampMs) });
}

function formatSessionCardSubtitle(session) {
  return `${session.working_width}x${session.working_height} | ${formatSessionStart(session.started_at)}`;
}

function formatRecordingFormatLabel(session) {
  return session && session.recording_format === "video-segments"
    ? "Video Session"
    : "Legacy Patch Session";
}

function formatTimelineLabel(timestampMs) {
  const offsetMs = Math.max(0, Number(timestampMs) - sessionStartMs());
  return `${formatElapsed(offsetMs)} | ${formatClockTime(timestampMs)}`;
}

function formatElapsed(durationMs) {
  const totalMs = Math.max(0, Number(durationMs) || 0);
  const totalSeconds = Math.floor(totalMs / 1000);
  const hours = Math.floor(totalSeconds / 3600);
  const minutes = Math.floor((totalSeconds % 3600) / 60);
  const seconds = totalSeconds % 60;

  if (hours > 0) {
    return state.language === "zh"
      ? `${hours}小时 ${minutes}分 ${seconds}秒`
      : `${hours}h ${minutes}m ${seconds}s`;
  }
  if (minutes > 0) {
    return state.language === "zh"
      ? `${minutes}分 ${seconds}秒`
      : `${minutes}m ${seconds}s`;
  }
  if (totalSeconds > 0) {
    return state.language === "zh" ? `${totalSeconds}秒` : `${totalSeconds}s`;
  }
  return state.language === "zh" ? `${totalMs}毫秒` : `${totalMs}ms`;
}

function formatClockTime(timestampMs) {
  const numeric = Number(timestampMs);
  if (!Number.isFinite(numeric) || numeric <= 0) {
    return String(timestampMs);
  }

  const locale = state.language === "zh" ? "zh-CN" : "en-US";
  const formatter = new Intl.DateTimeFormat(locale, {
    month: "2-digit",
    day: "2-digit",
    hour: "2-digit",
    minute: "2-digit",
    second: "2-digit",
    hour12: false,
  });
  return formatter.format(new Date(numeric));
}

function formatClockTimeWithDate(timestampMs) {
  const numeric = Number(timestampMs);
  if (!Number.isFinite(numeric) || numeric <= 0) {
    return String(timestampMs);
  }

  const locale = state.language === "zh" ? "zh-CN" : "en-US";
  const formatter = new Intl.DateTimeFormat(locale, {
    year: "numeric",
    month: "2-digit",
    day: "2-digit",
    hour: "2-digit",
    minute: "2-digit",
    second: "2-digit",
    hour12: false,
  });
  return formatter.format(new Date(numeric));
}

function sessionListState(session) {
  if (session.status) {
    return normalizeStatusState(session.status);
  }

  if (state.liveStatus && session.session_id === state.currentSessionId) {
    return normalizeStatusState(state.liveStatus);
  }

  return session.finished_at !== null && session.finished_at !== undefined ? "stopped" : "unknown";
}

function syncControlButtons(liveState) {
  const controlState = currentControlState(liveState);
  controlStartButton.disabled = controlState.startDisabled;
  controlPauseButton.disabled = controlState.pauseDisabled;
  controlResumeButton.disabled = controlState.resumeDisabled;
  controlStopButton.disabled = controlState.stopDisabled;
}

function setControlButtonsDisabled(disabled) {
  controlStartButton.disabled = disabled;
  controlPauseButton.disabled = disabled;
  controlResumeButton.disabled = disabled;
  controlStopButton.disabled = disabled;
}

function hasActiveRecordingSession() {
  return state.sessions.some((session) => {
    const liveState = sessionListState(session);
    return liveState === "running" || liveState === "paused";
  });
}

function currentControlState(currentLiveState = normalizeStatusState(state.liveStatus)) {
  return controlLogic.computeControlButtons({
    currentSessionId: state.currentSessionId,
    currentLiveState,
    sessions: state.sessions,
  });
}

async function waitForSessionAndSwitch(sessionId) {
  for (let attempt = 0; attempt < 10; attempt += 1) {
    await loadSessions();
    if (state.sessions.some((session) => session.session_id === sessionId)) {
      await switchToSession(sessionId);
      return;
    }
    await sleep(500);
  }

  await loadSessions();
}

async function switchToSession(sessionId) {
  stopPlayback();
  state.currentSessionId = sessionId;
  state.session = null;
  state.liveStatus = null;
  updateUrlSessionId(sessionId);
  await loadSession();
}

function sleep(ms) {
  return new Promise((resolve) => window.setTimeout(resolve, ms));
}

function normalizeStatusState(liveStatus) {
  if (liveStatus && typeof liveStatus.state === "string") {
    return liveStatus.state;
  }

  if (!liveStatus) {
    return "unknown";
  }

  return liveStatus.recording ? "running" : "stopped";
}

function formatStatusLabel(liveState) {
  switch (liveState) {
    case "running":
      return t("running");
    case "paused":
      return t("paused");
    case "stopped":
      return t("stopped");
    default:
      return t("unknown");
  }
}

function applyLanguage() {
  state.language = resolveLanguage();
  document.documentElement.lang = state.language === "zh" ? "zh-CN" : "en";
  document.title = t("appTitle");
  languageSelect.value = effectiveLanguagePreference();
  document.getElementById("viewer-title").textContent = t("appTitle");
  document.getElementById("timestamp-label").textContent = t("selectedTime");
  document.getElementById("timestamp-help").textContent = t("timestampHelp");
  document.getElementById("advanced-time-label").textContent = t("advancedTime");
  document.getElementById("timeline-title").textContent = t("timeline");
  document.getElementById("load").textContent = t("load");
  document.getElementById("prev").textContent = t("prev");
  document.getElementById("next").textContent = t("next");
  document.getElementById("speed-label").textContent = t("speed");
  document.getElementById("playback-loop-label").textContent = t("loopPlayback");
  const overlayLabel = document.getElementById("overlay-label");
  if (overlayLabel) {
    overlayLabel.textContent = t("overlay");
  }
  document.getElementById("language-label").textContent = t("language");
  document.getElementById("autostart-title").textContent = t("autostartTitle");
  document.getElementById("autostart-subtitle").textContent = t("autostartSubtitle");
  document.getElementById("autostart-enabled-label").textContent = t("autostartEnabledLabel");
  document.getElementById("autostart-login-label").textContent = t("autostartLoginLabel");
  document.getElementById("autostart-delay-label").textContent = t("autostartDelayLabel");
  document.getElementById("autostart-output-label").textContent = t("autostartOutputLabel");
  autostartRefreshButton.textContent = t("autostartRefresh");
  if (!autostartSaveButton.disabled) {
    autostartSaveButton.textContent = t("autostartSave");
  }
  document.getElementById("recording-settings-title").textContent = t("recordingSettingsTitle");
  document.getElementById("recording-settings-subtitle").textContent = t("recordingSettingsSubtitle");
  document.getElementById("recording-sampling-interval-label").textContent = t("recordingSamplingIntervalLabel");
  document.getElementById("recording-working-scale-label").textContent = t("recordingWorkingScaleLabel");
  document.getElementById("recording-burn-in-enabled-label").textContent = t("recordingBurnInEnabledLabel");
  recordingRefreshButton.textContent = t("recordingRefresh");
  if (!recordingSaveButton.disabled) {
    recordingSaveButton.textContent = t("recordingSave");
  }
  document.getElementById("quickstart-title").textContent = t("quickstartTitle");
  document.getElementById("quickstart-subtitle").textContent = t("quickstartSubtitle");
  document.getElementById("quickstart-step1-title").textContent = t("quickstartStep1Title");
  document.getElementById("quickstart-step1-body").textContent = t("quickstartStep1Body");
  document.getElementById("quickstart-step2-title").textContent = t("quickstartStep2Title");
  document.getElementById("quickstart-step2-body").textContent = t("quickstartStep2Body");
  document.getElementById("quickstart-step3-title").textContent = t("quickstartStep3Title");
  document.getElementById("quickstart-step3-body").textContent = t("quickstartStep3Body");
  document.getElementById("live-status-title").textContent = t("liveStatus");
  controlRefreshButton.textContent = t("refresh");
  controlStartButton.textContent = t("startRecording");
  controlPauseButton.textContent = t("pause");
  controlResumeButton.textContent = t("resume");
  controlStopButton.textContent = t("stop");
  document.getElementById("recent-sessions-title").textContent = t("recentSessions");
  document.getElementById("recent-sessions-subtitle").textContent = t("recentSessionsSubtitle");
  document.getElementById("session-filter-label").textContent = t("sessionFilter");
  deleteDaySessionsButton.textContent = t("deleteDaySessions");
  deleteAllSessionsButton.textContent = t("deleteAllSessions");
  refreshSessionsButton.textContent = t("refresh");
  playbackToggleButton.textContent = state.playbackRunning ? t("pausePlayback") : t("play");
  document.querySelector('#language-select option[value="auto"]').textContent = t("auto");
  document.querySelector('#language-select option[value="en"]').textContent = t("english");
  document.querySelector('#language-select option[value="zh"]').textContent = t("chinese");

  if (!state.session) {
    sessionInfo.textContent = t("sessionInfoLoading");
  } else {
    sessionInfo.textContent = t("currentSessionSummary", {
      timestamp: formatClockTimeWithDate(state.session.started_at),
      width: state.session.working_width,
      height: state.session.working_height,
    });
    sessionInfo.textContent = `${sessionInfo.textContent} | ${formatRecordingFormatLabel(state.session)}`;
  }
  if (!state.liveStatus) {
    recordingBadge.textContent = t("checking");
    statusSummary.textContent = t("waitingHeartbeat");
  }
  autostartNote.textContent = t("autostartNote");
  recordingSettingsNote.textContent = t("recordingNote");
  if (!state.autostart) {
    autostartState.textContent = t("autostartChecking");
  } else {
    renderAutostart();
  }
  renderAutostartFeedback();
  renderRecordingSettings();
  renderRecordingSettingsFeedback();
  renderLiveStatus();
  renderStatus();
  updateAdvancedTimeVisibility();
}

function t(key, vars = {}) {
  const message = (I18N[state.language] && I18N[state.language][key]) || I18N.en[key] || key;
  return message.replace(/\{(\w+)\}/g, (_, name) => String(vars[name] ?? ""));
}

function resolveLanguage() {
  const preference = effectiveLanguagePreference();
  if (preference === "en" || preference === "zh") {
    return preference;
  }
  return detectAutoLanguage();
}

function effectiveLanguagePreference() {
  return state.languagePreference || (state.session ? state.session.viewer_language : "auto") || "auto";
}

function detectAutoLanguage() {
  const language = (navigator.language || "").toLowerCase();
  return language.startsWith("zh") ? "zh" : "en";
}

function currentLanguagePreferenceFromUrl() {
  const params = new URLSearchParams(window.location.search);
  const value = params.get("lang");
  if (value === "auto" || value === "en" || value === "zh") {
    return value;
  }
  return null;
}

function loadStoredLanguagePreference() {
  const value = window.localStorage.getItem("viewerLanguage");
  if (value === "auto" || value === "en" || value === "zh") {
    return value;
  }
  return null;
}

function updateUrlLanguage(language) {
  const url = new URL(window.location.href);
  url.searchParams.set("lang", language);
  window.history.replaceState({}, "", url);
}

function formatStatusAction(action) {
  switch (action) {
    case "pause":
      return t("pause");
    case "resume":
      return t("resume");
    case "stop":
      return t("stop");
    default:
      return action;
  }
}

function updateAdvancedTimeVisibility() {
  timestampInput.classList.toggle("hidden", !advancedTimeToggle.checked);
}

function escapeHtml(value) {
  return String(value)
    .replaceAll("&", "&amp;")
    .replaceAll("<", "&lt;")
    .replaceAll(">", "&gt;")
    .replaceAll('"', "&quot;")
    .replaceAll("'", "&#39;");
}

function setStatusKey(key, vars = {}) {
  state.statusMessage = { kind: "i18n", key, vars };
  renderStatus();
}

function setRawStatus(message) {
  state.statusMessage = { kind: "raw", message };
  renderStatus();
}

function renderStatus() {
  if (!state.statusMessage) {
    status.textContent = "";
    return;
  }

  if (state.statusMessage.kind === "raw") {
    status.textContent = state.statusMessage.message;
    return;
  }

  status.textContent = t(state.statusMessage.key, state.statusMessage.vars);
}

loadSession().catch((err) => {
  setRawStatus(`Unexpected error: ${err.message}`);
});
