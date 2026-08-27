import { formatQuote, normalizeQuote } from "./quote.js";

const tauri = window.__TAURI__;

if (!tauri) {
  throw new Error("Tauri API is not available. Ensure withGlobalTauri is enabled.");
}

const { invoke } = tauri.core;
const { listen } = tauri.event;
const { open: shellOpen } = tauri.shell;

// ── i18n ──────────────────────────────────────────────────────────────────────

const STRINGS = {
  zh: {
    appDesc: "番茄时钟 · 休息锁定",
    aboutBtn: "关于",
    langToggle: "EN",
    startBtn: "开始番茄",
    pauseBtn: "暂停",
    resumeBtn: "继续",
    cancelBtn: "取消",
    phaseIdle: "待机",
    phaseFocus: "专注中",
    phaseBreak: "休息锁定",
    statusIdle: "准备开始新的专注轮次",
    statusFocus: "当前是工作阶段，不会锁定屏幕。",
    statusPaused: "已暂停，准备继续专注。",
    statusBreak: "休息阶段已开始，所有屏幕都处于锁定覆盖状态。",
    cyclesText: (n) => `今日完成 ${n} 轮`,
    quoteFallback: "“专注一点，世界会安静一些。”",
    settingsHeading: "偏好设置",
    settingsDesc: "调整休息行为与启动偏好。",
    durationLabel: "计时节奏",
    focusShortLabel: "专注",
    breakShortLabel: "休息",
    minuteUnit: "分",
    focusLabel: "工作时长（分钟）",
    breakLabel: "休息时长（分钟）",
    toggleOnlineQuote: "启用在线一言刷新",
    toggleWater: "展示喝水提醒",
    toggleStand: "展示站立活动提醒",
    toggleAutoRestart: "休息结束后自动开始下一轮",
    toggleForceBreak: "强制休息（不可提前结束）",
    toggleLaunchOnLogin: "开机自动启动",
    saveSettingsBtn: "保存设置",
    tasksHeading: "今日重要事项",
    tasksDesc: "最多 3 件，跨天自动清空。",
    taskPlaceholder: (i) => `事项 ${i + 1}`,
    saveTasksBtn: "保存事项",
    quoteLabel: "当前一言",
    sideNote:
      "工作时专注，休息时离开屏幕。休息锁定页会在所有显示器上展示今日重点、一言和提醒。",
    reminderWater: "喝水",
    reminderStand: "站立",
    reminderEye: "远眺",
    reminderShoulder: "放松肩颈",
    reminderLabel: "休息提醒",
    closeSettings: "关闭设置",
    aboutVersionPrefix: "版本",
    aboutDesc: "番茄时钟 + 休息强制锁定<br>工作时专注，休息时强制离开屏幕。",
    aboutGithub: "在 GitHub 上查看源码",
    lockTag: "放松时间",
    lockStatus: "离开桌面。喝水，站起来，等休息结束。",
    lockTasksHeading: "今日重要事项",
    lockQuoteHeading: "一言",
    lockFallbackTask: "休息结束后，回来处理今天最重要的一件事。",
    muteMediaBtn: "静音",
    unmuteMediaBtn: "恢复声音",
    endBreakBtn: "提前结束休息",
    waterReminder: "现在去喝一杯水，让眼睛离开屏幕。",
    standReminder: "站起来活动一下，肩颈和腿部都需要切换姿势。",
    defaultReminder: "休息几分钟，不要继续坐在屏幕前。",
    toggleStatus: "切换状态",
    errSaveSettings: (e) => `保存设置失败：${e}`,
    errSaveTasks: (e) => `保存事项失败：${e}`,
    errStart: (e) => `启动失败：${e}`,
    errOp: (e) => `操作失败：${e}`,
    errInit: (e) => `初始化失败：${e}`,
  },
  en: {
    appDesc: "Pomodoro · Break Lock",
    aboutBtn: "About",
    langToggle: "中文",
    startBtn: "Start",
    pauseBtn: "Pause",
    resumeBtn: "Resume",
    cancelBtn: "Cancel",
    phaseIdle: "Idle",
    phaseFocus: "Focusing",
    phaseBreak: "Break",
    statusIdle: "Ready to start a new focus session.",
    statusFocus: "Focus session in progress.",
    statusPaused: "Paused — ready to resume.",
    statusBreak: "Break started. All screens are locked.",
    cyclesText: (n) => `Today: ${n} round${n !== 1 ? "s" : ""}`,
    quoteFallback: '"Focus a little, and the world quiets down."',
    settingsHeading: "Preferences",
    settingsDesc: "Adjust break behavior and startup preferences.",
    durationLabel: "Timer durations",
    focusShortLabel: "Focus",
    breakShortLabel: "Break",
    minuteUnit: "min",
    focusLabel: "Focus (min)",
    breakLabel: "Break (min)",
    toggleOnlineQuote: "Enable online quote refresh",
    toggleWater: "Show water reminder",
    toggleStand: "Show stand-up reminder",
    toggleAutoRestart: "Auto-start next round after break",
    toggleForceBreak: "Force break (no early exit)",
    toggleLaunchOnLogin: "Launch on login",
    saveSettingsBtn: "Save Settings",
    tasksHeading: "Today's Focus",
    tasksDesc: "Up to 3 tasks. Auto-cleared each day.",
    taskPlaceholder: (i) => `Task ${i + 1}`,
    saveTasksBtn: "Save Tasks",
    quoteLabel: "Quote",
    sideNote:
      "Focus while working, step away during breaks. The lock screen shows your tasks, quote and reminders on all displays.",
    reminderWater: "Hydrate",
    reminderStand: "Stand",
    reminderEye: "Look away",
    reminderShoulder: "Stretch",
    reminderLabel: "Break reminders",
    closeSettings: "Close settings",
    aboutVersionPrefix: "Version",
    aboutDesc:
      "Pomodoro timer + forced break lock<br>Stay focused at work, step away during breaks.",
    aboutGithub: "View source on GitHub",
    lockTag: "Break Time",
    lockStatus: "Step away. Drink water, stand up, wait for the break to end.",
    lockTasksHeading: "Today's Focus",
    lockQuoteHeading: "Quote",
    lockFallbackTask: "When the break ends, come back and tackle the most important thing.",
    muteMediaBtn: "Mute",
    unmuteMediaBtn: "Restore Sound",
    endBreakBtn: "End Break Early",
    waterReminder: "Drink a glass of water and look away from the screen.",
    standReminder: "Stand up and move — your neck, shoulders and legs need a reset.",
    defaultReminder: "Rest for a few minutes. Step away from the screen.",
    toggleStatus: "Toggle status",
    errSaveSettings: (e) => `Failed to save settings: ${e}`,
    errSaveTasks: (e) => `Failed to save tasks: ${e}`,
    errStart: (e) => `Failed to start: ${e}`,
    errOp: (e) => `Operation failed: ${e}`,
    errInit: (e) => `Initialization failed: ${e}`,
  },
};

// ── State ──────────────────────────────────────────────────────────────────────

const state = {
  snapshot: null,
  reminderIndex: 0,
  reminderTimer: null,
  lang: "zh",
  appVersion: "",
  systemMuted: false,
  mutePending: false,
};

let T = STRINGS.zh;

// Dirty flags: when set, applySettings / applyTasks skip overwriting the form
// so edits-in-progress are not stomped by the per-second state push.
let durationDirty = false;
let preferencesDirty = false;
let durationRevision = 0;
let preferencesRevision = 0;
let tasksDirty = false;
let settingsSavedBeforeClose = false;

// ── Elements ───────────────────────────────────────────────────────────────────

const els = {
  appShell: document.querySelector("#app-shell"),
  lockShell: document.querySelector("#lock-shell"),
  langBtn: document.querySelector("#lang-btn"),
  settingsBtn: document.querySelector("#settings-btn"),
  settingsDialog: document.querySelector("#settings-dialog"),
  settingsClose: document.querySelector("#settings-close"),
  phasePill: document.querySelector("#phase-pill"),
  cycleText: document.querySelector("#cycle-text"),
  countdown: document.querySelector("#countdown"),
  statusText: document.querySelector("#status-text"),
  quoteCard: document.querySelector("#quote-card"),
  quoteAuthor: document.querySelector("#quote-author"),
  focusMinutes: document.querySelector("#focus-minutes"),
  breakMinutes: document.querySelector("#break-minutes"),
  onlineQuote: document.querySelector("#online-quote"),
  autoRestart: document.querySelector("#auto-restart"),
  forceBreak: document.querySelector("#force-break"),
  launchOnLogin: document.querySelector("#launch-on-login"),
  endBreakBtn: document.querySelector("#end-break-btn"),
  taskInputs: [0, 1, 2].map((index) => document.querySelector(`#task-${index}`)),
  statusBtns: [0, 1, 2].map((index) =>
    document.querySelector(`.status-btn[data-index="${index}"]`)
  ),
  startBtn: document.querySelector("#start-btn"),
  pauseBtn: document.querySelector("#pause-btn"),
  resumeBtn: document.querySelector("#resume-btn"),
  cancelBtn: document.querySelector("#cancel-btn"),
  saveSettingsBtn: document.querySelector("#save-settings-btn"),
  saveTasksBtn: document.querySelector("#save-tasks-btn"),
  lockTime: document.querySelector("#lock-time"),
  lockDate: document.querySelector("#lock-date"),
  lockCountdown: document.querySelector("#lock-countdown"),
  lockStatus: document.querySelector("#lock-status"),
  lockTasks: document.querySelector("#lock-tasks"),
  lockQuote: document.querySelector("#lock-quote"),
  lockReminder: document.querySelector("#lock-reminder"),
  muteMediaBtn: document.querySelector("#mute-media-btn"),
};

const isLockView = window.location.hash.startsWith("#lock");

// ── i18n helpers ───────────────────────────────────────────────────────────────

function effectiveLang(settings) {
  if (settings?.language === "en" || settings?.language === "zh") return settings.language;
  return navigator.language.startsWith("zh") ? "zh" : "en";
}

function setLang(lang) {
  state.lang = lang;
  T = STRINGS[lang] || STRINGS.zh;
  document.documentElement.lang = lang === "en" ? "en" : "zh-CN";
  applyI18n();
}

function applyI18n() {
  document.querySelectorAll("[data-i18n]").forEach((el) => {
    const val = T[el.dataset.i18n];
    if (val !== undefined) el.textContent = val;
  });
  document.querySelectorAll("[data-i18n-html]").forEach((el) => {
    const val = T[el.dataset.i18nHtml];
    if (val !== undefined) el.innerHTML = val;
  });
  document.querySelectorAll("[data-i18n-title]").forEach((el) => {
    const val = T[el.dataset.i18nTitle];
    if (val !== undefined) el.title = val;
  });
  document.querySelectorAll("[data-i18n-aria-label]").forEach((el) => {
    const val = T[el.dataset.i18nAriaLabel];
    if (val !== undefined) el.setAttribute("aria-label", val);
  });
  const versionEl = document.querySelector(".about-version");
  if (versionEl && state.appVersion) {
    versionEl.textContent = `${T.aboutVersionPrefix} ${state.appVersion}`;
  }
  if (isLockView) updateMuteUi();
  if (!isLockView && els.taskInputs[0]) {
    els.taskInputs.forEach((input, i) => {
      input.placeholder = T.taskPlaceholder(i);
    });
  }
}

function updateMuteUi() {
  if (!els.muteMediaBtn) return;
  els.muteMediaBtn.textContent = state.systemMuted ? T.unmuteMediaBtn : T.muteMediaBtn;
  els.muteMediaBtn.setAttribute("aria-pressed", String(state.systemMuted));
  els.muteMediaBtn.setAttribute("aria-busy", String(state.mutePending));
  els.muteMediaBtn.disabled = state.mutePending;
}

// ── Formatters ─────────────────────────────────────────────────────────────────

function formatDuration(totalSeconds) {
  const mins = Math.floor(totalSeconds / 60);
  const secs = totalSeconds % 60;
  return `${String(mins).padStart(2, "0")}:${String(secs).padStart(2, "0")}`;
}

function phaseLabel(phase) {
  if (phase === "focus") return T.phaseFocus;
  if (phase === "break") return T.phaseBreak;
  return T.phaseIdle;
}

function statusCopy(snapshot) {
  if (snapshot.phase === "focus") {
    return snapshot.paused ? T.statusPaused : T.statusFocus;
  }
  if (snapshot.phase === "break") return T.statusBreak;
  return T.statusIdle;
}

function quoteCopy(quote) {
  return formatQuote(quote, T.quoteFallback);
}

function reminderPool() {
  return [T.waterReminder, T.standReminder];
}

// ── Render ─────────────────────────────────────────────────────────────────────

function applySettings(snapshot) {
  if (!durationDirty) {
    els.focusMinutes.value = snapshot.settings.focusMinutes;
    els.breakMinutes.value = snapshot.settings.breakMinutes;
  }
  if (preferencesDirty) return;
  applyPreferenceInputs(snapshot);
}

function applyPreferenceInputs(snapshot) {
  els.onlineQuote.checked = snapshot.settings.enableOnlineQuote;
  els.autoRestart.checked = snapshot.settings.autoRestart;
  els.forceBreak.checked = snapshot.settings.forceBreak;
  els.launchOnLogin.checked = snapshot.settings.launchOnLogin;
}

const STATUS_CYCLE = { none: "doing", doing: "done", done: "none" };
const STATUS_ICON = { none: "○", doing: "◑", done: "✓" };

function applyTasks(snapshot) {
  if (!tasksDirty) {
    els.taskInputs.forEach((input, i) => {
      input.value = snapshot.todayTasks[i] ?? "";
    });
  }
  els.statusBtns.forEach((btn, i) => {
    // When dirty, check the live input value so the button enabled-state stays correct.
    const text = tasksDirty
      ? (els.taskInputs[i]?.value ?? "")
      : (snapshot.todayTasks[i] ?? "");
    const hasText = text.trim().length > 0;
    const status = hasText ? (snapshot.taskStatuses?.[i] ?? "none") : "none";
    btn.dataset.status = status;
    btn.textContent = STATUS_ICON[status] ?? "○";
    btn.disabled = !hasText;
    btn.closest(".task-item")?.setAttribute("data-status", hasText ? status : "none");
  });
}

function renderMain(snapshot) {
  els.phasePill.textContent = phaseLabel(snapshot.phase);
  els.cycleText.textContent = T.cyclesText(snapshot.completedCycles);
  els.countdown.textContent = formatDuration(snapshot.remainingSeconds || snapshot.totalSeconds);
  els.statusText.textContent = statusCopy(snapshot);
  const quote = normalizeQuote(snapshot.quote);
  els.quoteCard.textContent = quote ? formatQuote({ text: quote.text }, T.quoteFallback) : T.quoteFallback;
  els.quoteAuthor.textContent = quote?.author ? `— ${quote.author}` : "";

  els.startBtn.disabled = snapshot.phase !== "idle";
  els.pauseBtn.disabled = snapshot.phase === "idle" || snapshot.paused;
  els.resumeBtn.disabled = snapshot.phase === "idle" || !snapshot.paused;
  els.cancelBtn.disabled = snapshot.phase === "idle";
}

function renderLock(snapshot) {
  const reminders = reminderPool();
  els.lockCountdown.textContent = formatDuration(snapshot.remainingSeconds);
  els.lockStatus.textContent = T.lockStatus;
  els.lockQuote.textContent = quoteCopy(snapshot.quote);
  els.lockTasks.innerHTML = "";
  const rawTasks = snapshot.todayTasks || [];
  const rawStatuses = snapshot.taskStatuses || [];
  const filled = rawTasks
    .map((text, i) => ({ text, status: rawStatuses[i] ?? "none" }))
    .filter((t) => t.text.trim());
  const items = filled.length ? filled : [{ text: T.lockFallbackTask, status: "none" }];
  items.forEach(({ text, status }) => {
    const li = document.createElement("li");
    li.textContent = text;
    li.dataset.status = status;
    els.lockTasks.appendChild(li);
  });
  els.lockReminder.textContent = reminders[state.reminderIndex % reminders.length];
  els.endBreakBtn.hidden = snapshot.settings.forceBreak !== false;
}

function renderContent(snapshot) {
  if (!isLockView) {
    applySettings(snapshot);
    applyTasks(snapshot);
    renderMain(snapshot);
  } else {
    renderLock(snapshot);
  }
}

function render(snapshot) {
  const previousDate = state.snapshot?.todayDate;
  if (previousDate && snapshot.todayDate && previousDate !== snapshot.todayDate) {
    // A new business day must replace even unsaved values left in the inputs;
    // otherwise the dirty-edit guard would keep yesterday's tasks on screen.
    tasksDirty = false;
  }
  state.snapshot = snapshot;
  const lang = effectiveLang(snapshot.settings);
  if (state.lang !== lang) setLang(lang);
  renderContent(snapshot);
}

// ── Data ───────────────────────────────────────────────────────────────────────

async function loadSnapshot() {
  const snapshot = await invoke("get_snapshot");
  render(snapshot);
}

async function saveSettings({ durationRevisionAtStart, preferencesRevisionAtStart } = {}) {
  try {
    const settings = {
      focusMinutes: Number(els.focusMinutes.value) || 25,
      breakMinutes: Number(els.breakMinutes.value) || 5,
      enableOnlineQuote: els.onlineQuote.checked,
      autoRestart: els.autoRestart.checked,
      forceBreak: els.forceBreak.checked,
      launchOnLogin: els.launchOnLogin.checked,
      language: state.lang,
    };
    await invoke("save_settings", { settings });
    if (durationRevisionAtStart === durationRevision) durationDirty = false;
    if (preferencesRevisionAtStart === preferencesRevision) preferencesDirty = false;
    return true;
  } catch (err) {
    els.statusText.textContent = T.errSaveSettings(err);
    return false;
  }
}

function normalizeDuration(input, min, max, fallback) {
  const parsed = Number(input.value);
  const value = Number.isFinite(parsed)
    ? Math.min(max, Math.max(min, Math.round(parsed)))
    : fallback;
  input.value = String(value);
  return value;
}

async function persistTimerDurations() {
  const settings = state.snapshot?.settings;
  normalizeDuration(els.focusMinutes, 1, 180, settings?.focusMinutes ?? 25);
  normalizeDuration(els.breakMinutes, 1, 60, settings?.breakMinutes ?? 5);
  durationDirty = true;
  await saveSettings({ durationRevisionAtStart: durationRevision });
}

async function saveTasks() {
  try {
    const tasks = els.taskInputs.map((input) => input.value.trim());
    await invoke("set_today_tasks", { tasks });
    tasksDirty = false;
  } catch (err) {
    els.statusText.textContent = T.errSaveTasks(err);
  }
}

// ── Events ─────────────────────────────────────────────────────────────────────

function bindEvents() {
  if (!isLockView) {
    els.langBtn.addEventListener("click", async () => {
      const newLang = state.lang === "zh" ? "en" : "zh";
      setLang(newLang);
      if (state.snapshot) renderContent(state.snapshot);
      try {
        await invoke("save_settings", {
          settings: {
            focusMinutes: Number(els.focusMinutes.value) || 25,
            breakMinutes: Number(els.breakMinutes.value) || 5,
            enableOnlineQuote: els.onlineQuote.checked,
            autoRestart: els.autoRestart.checked,
            forceBreak: els.forceBreak.checked,
            launchOnLogin: els.launchOnLogin.checked,
            language: newLang,
          },
        });
      } catch (_) {}
    });

    // Mark settings/tasks dirty as soon as the user edits, so per-second
    // render calls don't stomp in-progress changes before the user saves.
    [els.focusMinutes, els.breakMinutes].forEach((el) =>
      el.addEventListener("input", () => {
        durationDirty = true;
        durationRevision += 1;
      })
    );
    [els.focusMinutes, els.breakMinutes].forEach((el) =>
      el.addEventListener("change", persistTimerDurations)
    );
    [els.onlineQuote, els.autoRestart, els.forceBreak, els.launchOnLogin].forEach((el) =>
      el.addEventListener("change", () => {
        preferencesDirty = true;
        preferencesRevision += 1;
      })
    );
    els.taskInputs.forEach((input) =>
      input.addEventListener("input", () => { tasksDirty = true; })
    );

    els.startBtn.addEventListener("click", async () => {
      try {
        const config = {
          focusMinutes: Number(els.focusMinutes.value) || 25,
          breakMinutes: Number(els.breakMinutes.value) || 5,
        };
        await invoke("start_pomodoro", { config });
      } catch (err) {
        els.statusText.textContent = T.errStart(err);
      }
    });

    els.pauseBtn.addEventListener("click", async () => {
      try { await invoke("pause_pomodoro"); } catch (err) { els.statusText.textContent = T.errOp(err); }
    });
    els.resumeBtn.addEventListener("click", async () => {
      try { await invoke("resume_pomodoro"); } catch (err) { els.statusText.textContent = T.errOp(err); }
    });
    els.cancelBtn.addEventListener("click", async () => {
      try { await invoke("cancel_pomodoro"); } catch (err) { els.statusText.textContent = T.errOp(err); }
    });
    els.saveSettingsBtn.addEventListener("click", async () => {
      const revision = preferencesRevision;
      if (await saveSettings({ preferencesRevisionAtStart: revision })) {
        if (revision !== preferencesRevision) return;
        settingsSavedBeforeClose = true;
        els.settingsDialog.close();
      }
    });
    els.saveTasksBtn.addEventListener("click", saveTasks);

    els.statusBtns.forEach((btn, i) => {
      btn.addEventListener("click", async () => {
        const next = STATUS_CYCLE[btn.dataset.status ?? "none"] ?? "none";
        try {
          await invoke("set_task_status", { index: i, status: next });
        } catch (err) {
          els.statusText.textContent = T.errOp(err);
        }
      });
    });

    const dialog = document.getElementById("about-dialog");
    els.settingsBtn.addEventListener("click", () => {
      if (dialog.open) dialog.close();
      preferencesDirty = false;
      if (state.snapshot) applyPreferenceInputs(state.snapshot);
      els.settingsDialog.showModal();
    });
    els.settingsClose.addEventListener("click", () => els.settingsDialog.close());
    els.settingsDialog.addEventListener("click", (event) => {
      if (event.target === els.settingsDialog) els.settingsDialog.close();
    });
    els.settingsDialog.addEventListener("close", () => {
      if (settingsSavedBeforeClose) {
        settingsSavedBeforeClose = false;
        return;
      }
      preferencesDirty = false;
      if (state.snapshot) applyPreferenceInputs(state.snapshot);
    });
    document.getElementById("about-btn").addEventListener("click", () => {
      if (els.settingsDialog.open) els.settingsDialog.close();
      dialog.showModal();
    });
    document.getElementById("about-close").addEventListener("click", () => dialog.close());
    document.getElementById("about-github").addEventListener("click", () =>
      shellOpen("https://github.com/acdiost/focus-lock/")
    );
    dialog.addEventListener("click", (e) => { if (e.target === dialog) dialog.close(); });
  } else {
    els.endBreakBtn.addEventListener("click", async () => {
      try { await invoke("exit_break_lock"); } catch (_) {}
    });
    els.muteMediaBtn.addEventListener("click", async () => {
      if (state.mutePending) return;
      const nextMuted = !state.systemMuted;
      state.mutePending = true;
      updateMuteUi();
      try {
        await invoke("set_system_mute", { muted: nextMuted });
      } catch (err) {
        els.muteMediaBtn.textContent = T.errOp(err);
        setTimeout(updateMuteUi, 1800);
      } finally {
        state.mutePending = false;
        els.muteMediaBtn.disabled = false;
        els.muteMediaBtn.setAttribute("aria-busy", "false");
      }
    });
    window.addEventListener("keydown", (event) => {
      if (event.key === "Escape" && state.snapshot?.settings.forceBreak === false) {
        event.preventDefault();
        invoke("exit_break_lock").catch(() => {});
        return;
      }
      if (["Escape", "F4"].includes(event.key) || event.metaKey || event.altKey) {
        event.preventDefault();
        event.stopPropagation();
      }
    });
    document.addEventListener("contextmenu", (event) => event.preventDefault());
  }
}

// ── Init ───────────────────────────────────────────────────────────────────────

async function init() {
  if (isLockView) {
    els.appShell.hidden = true;
    els.lockShell.hidden = false;
  } else {
    els.appShell.hidden = false;
    els.lockShell.hidden = true;
  }

  // Apply system language immediately to avoid a flash of untranslated content.
  setLang(navigator.language.startsWith("zh") ? "zh" : "en");

  // Fetch the authoritative version string from Tauri so the about dialog stays
  // in sync with Cargo.toml without requiring manual edits to the i18n strings.
  try {
    state.appVersion = await tauri.app.getVersion();
    applyI18n();
  } catch (_) {}

  await listen("system-mute://changed", (event) => {
    state.systemMuted = Boolean(event.payload);
    updateMuteUi();
  });
  bindEvents();
  await loadSnapshot();
  if (isLockView) {
    const updateClock = () => {
      const now = new Date();
      els.lockTime.textContent = now.toLocaleTimeString([], { hour: "2-digit", minute: "2-digit" });
      els.lockDate.textContent = now.toLocaleDateString([], { year: "numeric", month: "long", day: "numeric", weekday: "long" });
    };
    updateClock();
    setInterval(updateClock, 1000);

    state.reminderTimer = setInterval(() => {
      if (!state.snapshot) return;
      state.reminderIndex += 1;
      const reminders = reminderPool();
      els.lockReminder.textContent = reminders[state.reminderIndex % reminders.length];
    }, 6000);
  }
  await listen("pomodoro://state", (event) => {
    render(event.payload);
  });
  if (!isLockView) {
    await listen("open://about", () => {
      document.getElementById("about-dialog").showModal();
    });
  }
}

init().catch((error) => {
  console.error(error);
  if (!isLockView) {
    els.statusText.textContent = T.errInit(error);
  }
});
