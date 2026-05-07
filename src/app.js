const tauri = window.__TAURI__;

if (!tauri) {
  throw new Error("Tauri API is not available. Ensure withGlobalTauri is enabled.");
}

const { invoke } = tauri.tauri;
const { listen } = tauri.event;

const state = {
  snapshot: null,
  reminderIndex: 0,
  reminderTimer: null
};

const els = {
  appShell: document.querySelector("#app-shell"),
  lockShell: document.querySelector("#lock-shell"),
  phasePill: document.querySelector("#phase-pill"),
  cycleText: document.querySelector("#cycle-text"),
  countdown: document.querySelector("#countdown"),
  statusText: document.querySelector("#status-text"),
  quoteInline: document.querySelector("#quote-inline"),
  quoteCard: document.querySelector("#quote-card"),
  focusMinutes: document.querySelector("#focus-minutes"),
  breakMinutes: document.querySelector("#break-minutes"),
  onlineQuote: document.querySelector("#online-quote"),
  waterReminder: document.querySelector("#water-reminder"),
  standReminder: document.querySelector("#stand-reminder"),
  launchOnLogin: document.querySelector("#launch-on-login"),
  taskInputs: [0, 1, 2].map((index) => document.querySelector(`#task-${index}`)),
  startBtn: document.querySelector("#start-btn"),
  pauseBtn: document.querySelector("#pause-btn"),
  resumeBtn: document.querySelector("#resume-btn"),
  cancelBtn: document.querySelector("#cancel-btn"),
  saveSettingsBtn: document.querySelector("#save-settings-btn"),
  saveTasksBtn: document.querySelector("#save-tasks-btn"),
  lockCountdown: document.querySelector("#lock-countdown"),
  lockStatus: document.querySelector("#lock-status"),
  lockTasks: document.querySelector("#lock-tasks"),
  lockQuote: document.querySelector("#lock-quote"),
  lockReminder: document.querySelector("#lock-reminder")
};

const isLockView = window.location.hash.startsWith("#lock");

function formatDuration(totalSeconds) {
  const mins = Math.floor(totalSeconds / 60);
  const secs = totalSeconds % 60;
  return `${String(mins).padStart(2, "0")}:${String(secs).padStart(2, "0")}`;
}

function phaseLabel(phase) {
  if (phase === "focus") return "专注中";
  if (phase === "break") return "休息锁定";
  return "待机";
}

function statusCopy(snapshot) {
  if (snapshot.phase === "focus") {
    return snapshot.paused ? "已暂停，准备继续专注。" : "当前是工作阶段，不会锁定屏幕。";
  }
  if (snapshot.phase === "break") {
    return "休息阶段已开始，所有屏幕都处于锁定覆盖状态。";
  }
  return "准备开始新的专注轮次";
}

function quoteCopy(quote) {
  if (!quote) return "“专注一点，世界会安静一些。”";
  const meta = quote.author ? `\n—— ${quote.author}` : "";
  return `“${quote.text}”${meta}`;
}

function reminderPool(snapshot) {
  const reminders = [];
  if (snapshot.settings.enableWaterReminder) {
    reminders.push("现在去喝一杯水，让眼睛离开屏幕。");
  }
  if (snapshot.settings.enableStandReminder) {
    reminders.push("站起来活动一下，肩颈和腿部都需要切换姿势。");
  }
  if (reminders.length === 0) {
    reminders.push("休息几分钟，不要继续坐在屏幕前。");
  }
  return reminders;
}

function applySettings(snapshot) {
  els.focusMinutes.value = snapshot.settings.focusMinutes;
  els.breakMinutes.value = snapshot.settings.breakMinutes;
  els.onlineQuote.checked = snapshot.settings.enableOnlineQuote;
  els.waterReminder.checked = snapshot.settings.enableWaterReminder;
  els.standReminder.checked = snapshot.settings.enableStandReminder;
  els.launchOnLogin.checked = snapshot.settings.launchOnLogin;
}

function applyTasks(snapshot) {
  els.taskInputs.forEach((input, index) => {
    input.value = snapshot.todayTasks[index] ?? "";
  });
}

function renderMain(snapshot) {
  els.phasePill.textContent = phaseLabel(snapshot.phase);
  els.cycleText.textContent = `今日完成 ${snapshot.completedCycles} 轮`;
  els.countdown.textContent = formatDuration(snapshot.remainingSeconds || snapshot.totalSeconds);
  els.statusText.textContent = statusCopy(snapshot);
  els.quoteInline.textContent = snapshot.quote ? `“${snapshot.quote.text}”` : "“专注一点，世界会安静一些。”";
  els.quoteCard.textContent = quoteCopy(snapshot.quote);

  els.startBtn.disabled = snapshot.phase !== "idle";
  els.pauseBtn.disabled = snapshot.phase === "idle" || snapshot.paused;
  els.resumeBtn.disabled = snapshot.phase === "idle" || !snapshot.paused;
  els.cancelBtn.disabled = snapshot.phase === "idle";
}

function renderLock(snapshot) {
  const reminders = reminderPool(snapshot);
  els.lockCountdown.textContent = formatDuration(snapshot.remainingSeconds);
  els.lockStatus.textContent = "离开桌面。喝水，站起来，等休息结束。";
  els.lockQuote.textContent = quoteCopy(snapshot.quote);
  els.lockTasks.innerHTML = "";
  const items = snapshot.todayTasks.length ? snapshot.todayTasks : ["休息结束后，回来处理今天最重要的一件事。"];
  items.forEach((task) => {
    const li = document.createElement("li");
    li.textContent = task;
    els.lockTasks.appendChild(li);
  });
  els.lockReminder.textContent = reminders[state.reminderIndex % reminders.length];

  if (state.reminderTimer) {
    clearInterval(state.reminderTimer);
  }
  state.reminderTimer = setInterval(() => {
    state.reminderIndex += 1;
    els.lockReminder.textContent = reminders[state.reminderIndex % reminders.length];
  }, 6000);
}

function render(snapshot) {
  state.snapshot = snapshot;
  if (!isLockView) {
    applySettings(snapshot);
    applyTasks(snapshot);
    renderMain(snapshot);
  } else {
    renderLock(snapshot);
  }
}

async function loadSnapshot() {
  const snapshot = await invoke("get_snapshot");
  render(snapshot);
}

async function saveSettings() {
  const settings = {
    focusMinutes: Number(els.focusMinutes.value) || 25,
    breakMinutes: Number(els.breakMinutes.value) || 5,
    enableOnlineQuote: els.onlineQuote.checked,
    enableWaterReminder: els.waterReminder.checked,
    enableStandReminder: els.standReminder.checked,
    launchOnLogin: els.launchOnLogin.checked
  };
  await invoke("save_settings", { settings });
}

async function saveTasks() {
  const tasks = els.taskInputs.map((input) => input.value.trim());
  await invoke("set_today_tasks", { tasks });
}

function bindEvents() {
  if (!isLockView) {
    els.startBtn.addEventListener("click", async () => {
      const config = {
        focusMinutes: Number(els.focusMinutes.value) || 25,
        breakMinutes: Number(els.breakMinutes.value) || 5
      };
      await invoke("start_pomodoro", { config });
    });

    els.pauseBtn.addEventListener("click", async () => invoke("pause_pomodoro"));
    els.resumeBtn.addEventListener("click", async () => invoke("resume_pomodoro"));
    els.cancelBtn.addEventListener("click", async () => invoke("cancel_pomodoro"));
    els.saveSettingsBtn.addEventListener("click", saveSettings);
    els.saveTasksBtn.addEventListener("click", saveTasks);
  } else {
    window.addEventListener("keydown", (event) => {
      if (["Escape", "F4"].includes(event.key) || event.metaKey || event.altKey) {
        event.preventDefault();
        event.stopPropagation();
      }
    });
    document.addEventListener("contextmenu", (event) => event.preventDefault());
  }
}

async function init() {
  if (isLockView) {
    els.appShell.hidden = true;
    els.lockShell.hidden = false;
  } else {
    els.appShell.hidden = false;
    els.lockShell.hidden = true;
  }

  bindEvents();
  await loadSnapshot();
  await listen("pomodoro://state", (event) => {
    render(event.payload);
  });
}

init().catch((error) => {
  console.error(error);
  if (!isLockView) {
    els.statusText.textContent = `初始化失败：${error}`;
  }
});
