#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
#![allow(unexpected_cfgs)]

use std::{
    fs,
    path::PathBuf,
    sync::Mutex,
    thread,
    time::{Duration, Instant},
};

use chrono::Local;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use tauri::{
    Emitter, Manager, RunEvent, WebviewUrl, WebviewWindowBuilder, WindowEvent,
};
use tauri::menu::{Menu, MenuItem, PredefinedMenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};

type AppHandle = tauri::AppHandle<tauri::Wry>;

const STATE_FILE_NAME: &str = "state.json";
const LOCK_PREFIX: &str = "lock-screen-";
const ONLINE_QUOTE_ENDPOINT: &str = "https://v1.hitokoto.cn/?encode=json";

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Settings {
    focus_minutes: u64,
    break_minutes: u64,
    enable_online_quote: bool,
    auto_restart: bool,
    #[serde(default)]
    launch_on_login: bool,
    #[serde(default = "default_true")]
    force_break: bool,
    #[serde(default)]
    language: String,
}

fn default_true() -> bool { true }

impl Default for Settings {
    fn default() -> Self {
        Self {
            focus_minutes: 25,
            break_minutes: 5,
            enable_online_quote: true,
            auto_restart: false,
            launch_on_login: false,
            force_break: true,
            language: String::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Quote {
    text: String,
    author: Option<String>,
    source: String,
    fetched_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct PersistentState {
    settings: Settings,
    today_date: String,
    today_tasks: Vec<String>,
    #[serde(default)]
    task_statuses: Vec<String>,
    quote_cache: Option<Quote>,
    completed_cycles: u32,
}

impl Default for PersistentState {
    fn default() -> Self {
        Self {
            settings: Settings::default(),
            today_date: today_string(),
            today_tasks: vec![String::new(); 3],
            task_statuses: vec!["none".to_string(); 3],
            quote_cache: None,
            completed_cycles: 0,
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
enum Phase {
    Idle,
    Focus,
    Break,
}

#[derive(Debug, Clone)]
struct RuntimeState {
    phase: Phase,
    remaining_seconds: u64,
    total_seconds: u64,
    paused: bool,
    last_tick: Instant,
    current_quote: Quote,
    allow_lock_close: bool,
}

impl Default for RuntimeState {
    fn default() -> Self {
        Self {
            phase: Phase::Idle,
            remaining_seconds: 0,
            total_seconds: 0,
            paused: false,
            last_tick: Instant::now(),
            current_quote: local_quote_for_seed(0),
            allow_lock_close: false,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct Snapshot {
    phase: Phase,
    remaining_seconds: u64,
    total_seconds: u64,
    paused: bool,
    completed_cycles: u32,
    settings: Settings,
    today_tasks: Vec<String>,
    task_statuses: Vec<String>,
    quote: Quote,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct StartConfig {
    focus_minutes: u64,
    break_minutes: u64,
}

#[derive(Debug, Deserialize)]
struct HitokotoResponse {
    hitokoto: String,
    from: Option<String>,
    from_who: Option<String>,
}

#[derive(Default)]
struct AppState {
    persistent: Mutex<PersistentState>,
    runtime: Mutex<RuntimeState>,
    last_tray_key: Mutex<String>,
}

fn today_string() -> String {
    Local::now().format("%Y-%m-%d").to_string()
}

fn state_file_path(app: &AppHandle) -> PathBuf {
    let mut root = app.path().app_config_dir().unwrap_or_else(|_| PathBuf::from("."));
    root.push("Focus Lock");
    root.push(STATE_FILE_NAME);
    root
}

fn ensure_today_fresh(persistent: &mut PersistentState) {
    let today = today_string();
    if persistent.today_date != today {
        persistent.today_date = today;
        persistent.today_tasks = vec![String::new(); 3];
        persistent.task_statuses = vec!["none".to_string(); 3];
        persistent.completed_cycles = 0;
    }
    persistent.today_tasks.resize(3, String::new());
    persistent.task_statuses.resize(3, "none".to_string());
}

fn load_persistent_state(app: &AppHandle) -> PersistentState {
    let file = state_file_path(app);
    if let Ok(contents) = fs::read_to_string(&file) {
        if let Ok(mut state) = serde_json::from_str::<PersistentState>(&contents) {
            ensure_today_fresh(&mut state);
            return state;
        }
    }
    PersistentState::default()
}

fn persist_state(app: &AppHandle, state: &PersistentState) -> tauri::Result<()> {
    let file = state_file_path(app);
    if let Some(parent) = file.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(file, serde_json::to_vec_pretty(state)?)?;
    Ok(())
}

fn local_quote_seed() -> usize {
    today_string()
        .bytes()
        .fold(0usize, |acc, b| acc.wrapping_mul(31).wrapping_add(b as usize))
}

fn local_quotes() -> &'static [(&'static str, &'static str)] {
    &[
        ("先完成最重要的一件事，剩下的噪音会自己退后。", "Focus Lock"),
        ("专注不是压榨自己，而是替注意力清场。", "Focus Lock"),
        ("屏幕之外的几分钟休息，会换回来更长的稳定输出。", "Focus Lock"),
        ("不要把休息当中断，它是下一段专注的准备动作。", "Focus Lock"),
        ("今天真正重要的事情，通常不会超过三件。", "Focus Lock"),
        ("把目光从像素里拔出来，身体才会跟上你的节奏。", "Focus Lock"),
        ("好的节奏不是一直冲刺，而是知道什么时候停下来。", "Focus Lock"),
    ]
}

fn local_quote_for_seed(seed: usize) -> Quote {
    let quotes = local_quotes();
    let (text, author) = quotes[seed % quotes.len()];
    Quote {
        text: text.to_string(),
        author: Some(author.to_string()),
        source: "local".to_string(),
        fetched_at: None,
    }
}

fn build_snapshot(app: &AppHandle) -> Snapshot {
    let state = app.state::<AppState>();
    let mut persistent = state.persistent.lock().unwrap();
    ensure_today_fresh(&mut persistent);
    let runtime = state.runtime.lock().unwrap();
    Snapshot {
        phase: runtime.phase,
        remaining_seconds: runtime.remaining_seconds,
        total_seconds: runtime.total_seconds,
        paused: runtime.paused,
        completed_cycles: persistent.completed_cycles,
        settings: persistent.settings.clone(),
        today_tasks: persistent.today_tasks.clone(),
        task_statuses: persistent.task_statuses.clone(),
        quote: runtime.current_quote.clone(),
    }
}

fn build_tray_menu(
    app: &AppHandle,
    phase: Phase,
    paused: bool,
    cycles: u32,
    force_break: bool,
    tasks: &[(String, String)],
    lang: &str,
) -> tauri::Result<Menu<tauri::Wry>> {
    let en = lang == "en";

    let cycle_suffix = if cycles > 0 {
        if en {
            let unit = if cycles == 1 { "round" } else { "rounds" };
            format!(" · Today: {} {}", cycles, unit)
        } else {
            format!(" · 今日 {} 轮", cycles)
        }
    } else {
        String::new()
    };

    let status = match phase {
        Phase::Idle => format!("Focus Lock · {}{}", if en { "Idle" } else { "待机" }, cycle_suffix),
        Phase::Focus if paused => format!("{}{}", if en { "Paused" } else { "专注暂停" }, cycle_suffix),
        Phase::Focus => format!("{}{}", if en { "Focusing" } else { "专注中" }, cycle_suffix),
        Phase::Break => format!("{}{}", if en { "Break" } else { "休息中" }, cycle_suffix),
    };

    let menu = Menu::new(app)?;
    menu.append(&MenuItem::with_id(app, "status", &status, false, None::<&str>)?)?;
    menu.append(&PredefinedMenuItem::separator(app)?)?;

    let has_tasks = tasks.iter().any(|(t, _)| !t.is_empty());
    if has_tasks {
        for (slot, (text, status_str)) in tasks.iter().enumerate() {
            if text.is_empty() {
                continue;
            }
            let icon = match status_str.as_str() {
                "doing" => "◑",
                "done" => "✓",
                _ => "○",
            };
            let chars_count = text.chars().count();
            let label_text: String = text.chars().take(16).collect();
            let label_text = if chars_count > 16 { format!("{}…", label_text) } else { label_text };
            menu.append(&MenuItem::with_id(
                app,
                format!("task_{}", slot),
                format!("{} {}", icon, label_text),
                true,
                None::<&str>,
            )?)?;
        }
        menu.append(&PredefinedMenuItem::separator(app)?)?;
    }

    match phase {
        Phase::Idle => {
            menu.append(&MenuItem::with_id(app, "start", if en { "Start" } else { "开始番茄" }, true, None::<&str>)?)?;
        }
        Phase::Focus if paused => {
            menu.append(&MenuItem::with_id(app, "resume", if en { "Resume" } else { "继续专注" }, true, None::<&str>)?)?;
            menu.append(&MenuItem::with_id(app, "cancel", if en { "Cancel Round" } else { "取消本轮" }, true, None::<&str>)?)?;
        }
        Phase::Focus => {
            menu.append(&MenuItem::with_id(app, "pause", if en { "Pause" } else { "暂停专注" }, true, None::<&str>)?)?;
            menu.append(&MenuItem::with_id(app, "cancel", if en { "Cancel Round" } else { "取消本轮" }, true, None::<&str>)?)?;
        }
        Phase::Break if !force_break => {
            menu.append(&MenuItem::with_id(app, "end_break", if en { "End Break Early" } else { "提前结束休息" }, true, None::<&str>)?)?;
        }
        Phase::Break => {}
    }

    menu.append(&PredefinedMenuItem::separator(app)?)?;
    menu.append(&MenuItem::with_id(app, "show", if en { "Show Window" } else { "显示主窗口" }, true, None::<&str>)?)?;
    menu.append(&MenuItem::with_id(app, "about", if en { "About Focus Lock" } else { "关于 Focus Lock" }, true, None::<&str>)?)?;
    menu.append(&PredefinedMenuItem::separator(app)?)?;
    menu.append(&MenuItem::with_id(app, "quit", if en { "Quit" } else { "退出" }, true, None::<&str>)?)?;

    Ok(menu)
}

fn update_tray_menu(app: &AppHandle) {
    let state = app.state::<AppState>();
    let (phase, paused) = {
        let r = state.runtime.lock().unwrap();
        (r.phase, r.paused)
    };
    let (cycles, force_break, tasks, language) = {
        let p = state.persistent.lock().unwrap();
        let tasks: Vec<(String, String)> = p
            .today_tasks.iter().zip(p.task_statuses.iter())
            .map(|(t, s)| (t.clone(), s.clone()))
            .collect();
        (p.completed_cycles, p.settings.force_break, tasks, p.settings.language.clone())
    };

    let tasks_key: String = tasks.iter().map(|(t, s)| format!("{t}:{s}|")).collect();
    let new_key = format!("{phase:?}|{paused}|{cycles}|{force_break}|{language}|{tasks_key}");
    {
        let mut last_key = state.last_tray_key.lock().unwrap();
        if *last_key == new_key {
            return;
        }
        *last_key = new_key;
    }

    if let Ok(menu) = build_tray_menu(app, phase, paused, cycles, force_break, &tasks, &language) {
        if let Some(tray) = app.tray_by_id("main-tray") {
            let _ = tray.set_menu(Some(menu));
        }
    }
}

fn emit_snapshot(app: &AppHandle) {
    let snapshot = build_snapshot(app);
    let _ = app.emit("pomodoro://state", snapshot);
    update_tray_menu(app);
}

// ── Launch at login ─────────────────────────────────────

#[cfg(target_os = "macos")]
fn apply_launch_at_login(enable: bool) {
    let Some(home) = std::env::var("HOME").ok() else { return };
    let plist_path = std::path::PathBuf::from(home)
        .join("Library/LaunchAgents/com.acdiost.focuslock.plist");

    if enable {
        let Ok(exe) = std::env::current_exe() else { return };
        let exe_str = exe.to_string_lossy();
        let plist = format!(
            "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n\
             <!DOCTYPE plist PUBLIC \"-//Apple//DTD PLIST 1.0//EN\" \
             \"http://www.apple.com/DTDs/PropertyList-1.0.dtd\">\n\
             <plist version=\"1.0\">\n\
             <dict>\n\
             \t<key>Label</key>\n\
             \t<string>com.acdiost.focuslock</string>\n\
             \t<key>ProgramArguments</key>\n\
             \t<array>\n\
             \t\t<string>{exe_str}</string>\n\
             \t</array>\n\
             \t<key>RunAtLoad</key>\n\
             \t<true/>\n\
             \t<key>KeepAlive</key>\n\
             \t<false/>\n\
             </dict>\n\
             </plist>\n"
        );
        if let Some(dir) = plist_path.parent() {
            let _ = fs::create_dir_all(dir);
        }
        let _ = fs::write(&plist_path, plist);
    } else {
        let _ = fs::remove_file(&plist_path);
    }
}

#[cfg(target_os = "windows")]
fn apply_launch_at_login(enable: bool) {
    use winreg::enums::{HKEY_CURRENT_USER, KEY_SET_VALUE};
    use winreg::RegKey;

    let Ok(run_key) = RegKey::predef(HKEY_CURRENT_USER).open_subkey_with_flags(
        "SOFTWARE\\Microsoft\\Windows\\CurrentVersion\\Run",
        KEY_SET_VALUE,
    ) else {
        return;
    };

    if enable {
        if let Ok(exe) = std::env::current_exe() {
            let _ = run_key.set_value("Focus Lock", &exe.to_string_lossy().as_ref());
        }
    } else {
        let _ = run_key.delete_value("Focus Lock");
    }
}

#[cfg(target_os = "linux")]
fn apply_launch_at_login(enable: bool) {
    let Some(home) = std::env::var("HOME").ok() else { return };
    let autostart_dir = std::path::PathBuf::from(home).join(".config/autostart");
    let desktop_path = autostart_dir.join("com.acdiost.focuslock.desktop");

    if enable {
        let Ok(exe) = std::env::current_exe() else { return };
        let exe_str = exe.to_string_lossy();
        let content = format!(
            "[Desktop Entry]\n\
             Type=Application\n\
             Name=Focus Lock\n\
             Comment=Pomodoro timer with break-time screen lock\n\
             Exec={exe_str}\n\
             Icon=com.acdiost.focuslock\n\
             Hidden=false\n\
             NoDisplay=false\n\
             X-GNOME-Autostart-enabled=true\n"
        );
        let _ = fs::create_dir_all(&autostart_dir);
        let _ = fs::write(&desktop_path, content);
    } else {
        let _ = fs::remove_file(&desktop_path);
    }
}

#[cfg(not(any(target_os = "macos", target_os = "windows", target_os = "linux")))]
fn apply_launch_at_login(_enable: bool) {}

fn current_quote(persistent: &PersistentState) -> Quote {
    persistent
        .quote_cache
        .clone()
        .unwrap_or_else(|| local_quote_for_seed(local_quote_seed()))
}

// Synchronously hide the macOS menu bar and dock before any lock window is shown.
// Blocks the calling (timer) thread until the main thread has applied the options.
// Presentation flags 186 =
// HideDock(2)|HideMenuBar(8)|DisableAppleMenu(16)|DisableForceQuit(32)|DisableHideApplication(128)
#[cfg(target_os = "macos")]
fn macos_hide_chrome_sync(app: &AppHandle) {
    let (tx, rx) = std::sync::mpsc::channel::<()>();
    let _ = app.run_on_main_thread(move || unsafe {
        #[allow(unused_imports)]
        use objc::{class, msg_send, sel, sel_impl};
        use objc::runtime::Object;
        let ns_app: *mut Object = msg_send![class!(NSApplication), sharedApplication];
        let _: () = msg_send![ns_app, setPresentationOptions: 186_u64];
        let _ = tx.send(());
    });
    let _ = rx.recv();
}

#[cfg(not(target_os = "macos"))]
fn macos_hide_chrome_sync(_app: &AppHandle) {}

// Configure and show all lock windows in a single main-thread dispatch.
// Called after macos_hide_chrome_sync so the menu bar is already gone.
// Window level 1000 = NSScreenSaverWindowLevel.
// Collection behavior 81 = CanJoinAllSpaces(1)|Stationary(16)|IgnoresCycle(64).
// Background #fdf9f5 matches the CSS lock-shell background exactly.
#[cfg(target_os = "macos")]
fn macos_show_locks(app: &AppHandle, labels: Vec<String>) {
    let app2 = app.clone();
    let _ = app.run_on_main_thread(move || unsafe {
        #[allow(unused_imports)]
        use objc::{class, msg_send, sel, sel_impl};
        use objc::runtime::Object;

        let bg: *mut Object = msg_send![class!(NSColor),
            colorWithRed: 0.992_f64
            green:        0.976_f64
            blue:         0.961_f64
            alpha:        1.0_f64];

        // Phase 1: configure all windows before any becomes visible.
        let mut ptrs: Vec<*mut Object> = Vec::new();
        for label in &labels {
            let Some(w) = app2.get_webview_window(label) else { continue };
            let Ok(raw) = w.ns_window() else { continue };
            let ptr = raw as *mut Object;
            let _: () = msg_send![ptr, setBackgroundColor: bg];
            let _: () = msg_send![ptr, setOpaque: 1_i8];
            let _: () = msg_send![ptr, setCollectionBehavior: 81_u64];
            let _: () = msg_send![ptr, setLevel: 1000_i64];
            ptrs.push(ptr);
        }

        // Phase 2: show all windows now that levels are set.
        for &ptr in &ptrs {
            let _: () = msg_send![ptr, orderFrontRegardless];
        }

        // Phase 3: keyboard focus on the primary monitor's window.
        if let Some(&ptr) = ptrs.first() {
            let _: () = msg_send![ptr, makeKeyWindow];
        }
    });
}

#[cfg(not(target_os = "macos"))]
fn macos_show_locks(_app: &AppHandle, _labels: Vec<String>) {}

// Re-apply NSScreenSaverWindowLevel after a focus loss that may have been
// triggered by Tauri's set_always_on_top() resetting the level to 3.
#[cfg(target_os = "macos")]
fn macos_relevel_lock(app: &AppHandle, label: &str) {
    let app2 = app.clone();
    let label = label.to_string();
    let _ = app.run_on_main_thread(move || unsafe {
        #[allow(unused_imports)]
        use objc::{class, msg_send, sel, sel_impl};
        use objc::runtime::Object;
        if let Some(w) = app2.get_webview_window(&label) {
            if let Ok(ptr) = w.ns_window() {
                let _: () = msg_send![ptr as *mut Object, setLevel: 1000_i64];
                let _: () = msg_send![ptr as *mut Object, orderFrontRegardless];
                let _: () = msg_send![ptr as *mut Object, makeKeyWindow];
            }
        }
    });
}

#[cfg(not(target_os = "macos"))]
fn macos_relevel_lock(_app: &AppHandle, _label: &str) {}

#[cfg(target_os = "macos")]
fn macos_lock_end(app: &AppHandle) {
    let _ = app.run_on_main_thread(|| unsafe {
        #[allow(unused_imports)]
        use objc::{class, msg_send, sel, sel_impl};
        use objc::runtime::Object;
        let ns_app: *mut Object = msg_send![class!(NSApplication), sharedApplication];
        let _: () = msg_send![ns_app, setPresentationOptions: 0_u64];
    });
}

#[cfg(not(target_os = "macos"))]
fn macos_lock_end(_app: &AppHandle) {}

fn close_lock_windows(app: &AppHandle) {
    {
        let state = app.state::<AppState>();
        let mut runtime = state.runtime.lock().unwrap();
        runtime.allow_lock_close = true;
    }

    let labels: Vec<String> = app
        .webview_windows()
        .keys()
        .filter(|label| label.starts_with(LOCK_PREFIX))
        .cloned()
        .collect();

    for label in labels {
        if let Some(window) = app.get_webview_window(&label) {
            let _ = window.close();
        }
    }

    macos_lock_end(app);
}

fn sync_lock_windows(app: &AppHandle) -> tauri::Result<()> {
    close_lock_windows(app);

    {
        let state = app.state::<AppState>();
        let mut runtime = state.runtime.lock().unwrap();
        runtime.allow_lock_close = false;
    }

    let Some(main_window) = app.get_webview_window("main") else {
        return Ok(());
    };
    let monitors = main_window.available_monitors()?;

    // On macOS: hide menu bar and dock before any window is created so there is
    // no frame where the system chrome is visible behind the lock windows.
    macos_hide_chrome_sync(app);

    let mut labels: Vec<String> = Vec::new();
    for (index, monitor) in monitors.into_iter().enumerate() {
        let label = format!("{LOCK_PREFIX}{index}");
        let size = monitor.size();
        let position = monitor.position();
        let window =
            WebviewWindowBuilder::new(app, label.clone(), WebviewUrl::App("index.html#lock".into()))
                .decorations(false)
                .resizable(false)
                .skip_taskbar(true)
                .visible(false)
                .title("Focus Lock Break")
                .shadow(false)
                .inner_size(1.0, 1.0)
                .position(0.0, 0.0)
                .build()?;

        // Correct with exact physical-pixel values: avoids 1-2px rounding errors
        // caused by dividing physical coords by a fractional scale factor, which
        // on Windows dual-monitor setups with mismatched DPI causes edge flicker.
        let _ = window.set_size(*size);
        let _ = window.set_position(*position);

        // On non-macOS platforms, use Tauri's window APIs directly.
        #[cfg(not(target_os = "macos"))]
        {
            let _ = window.set_always_on_top(true);
            let _ = window.set_focus();
            let _ = window.show();
            // On Linux, _NET_WM_STATE_ABOVE is not enough to cover dock/panel windows
            // (_NET_WM_WINDOW_TYPE_DOCK). Setting fullscreen triggers
            // _NET_WM_STATE_FULLSCREEN which X11/Mutter places above all panels.
            #[cfg(target_os = "linux")]
            let _ = window.set_fullscreen(true);
        }

        labels.push(label);
    }

    // On macOS: configure NSWindow level + show all windows in one main-thread
    // dispatch so no intermediate state is visible.
    macos_show_locks(app, labels);

    Ok(())
}

async fn refresh_online_quote(app: AppHandle) {
    let should_fetch = {
        let state = app.state::<AppState>();
        let persistent = state.persistent.lock().unwrap();
        if !persistent.settings.enable_online_quote {
            false
        } else {
            let cache_fresh = persistent
                .quote_cache
                .as_ref()
                .and_then(|q| q.fetched_at.as_deref())
                .and_then(|ts| chrono::DateTime::parse_from_rfc3339(ts).ok())
                .map(|fetched| Local::now().signed_duration_since(fetched).num_hours() < 24)
                .unwrap_or(false);
            !cache_fresh
        }
    };

    if !should_fetch {
        return;
    }

    let client = match Client::builder().timeout(Duration::from_secs(3)).build() {
        Ok(client) => client,
        Err(_) => return,
    };

    let response = match client.get(ONLINE_QUOTE_ENDPOINT).send().await {
        Ok(response) => response,
        Err(_) => return,
    };

    let payload = match response.json::<HitokotoResponse>().await {
        Ok(payload) => payload,
        Err(_) => return,
    };

    let quote = Quote {
        text: payload.hitokoto,
        author: payload.from_who.or(payload.from),
        source: "online".to_string(),
        fetched_at: Some(Local::now().to_rfc3339()),
    };

    let state = app.state::<AppState>();
    {
        let mut persistent = state.persistent.lock().unwrap();
        persistent.quote_cache = Some(quote.clone());
        let _ = persist_state(&app, &persistent);
    }
    {
        let mut runtime = state.runtime.lock().unwrap();
        runtime.current_quote = quote;
    }
    emit_snapshot(&app);
}

fn begin_focus(app: &AppHandle, focus_minutes: u64, break_minutes: u64) -> tauri::Result<Snapshot> {
    let state = app.state::<AppState>();
    {
        let mut persistent = state.persistent.lock().unwrap();
        ensure_today_fresh(&mut persistent);
        persistent.settings.focus_minutes = focus_minutes.clamp(1, 180);
        persistent.settings.break_minutes = break_minutes.clamp(1, 60);
        persist_state(app, &persistent)?;
    }
    {
        let mut runtime = state.runtime.lock().unwrap();
        runtime.phase = Phase::Focus;
        runtime.remaining_seconds = focus_minutes.clamp(1, 180) * 60;
        runtime.total_seconds = runtime.remaining_seconds;
        runtime.paused = false;
        runtime.last_tick = Instant::now();
    }
    emit_snapshot(app);
    Ok(build_snapshot(app))
}

fn transition_to_break(app: &AppHandle) {
    {
        let state = app.state::<AppState>();
        let mut persistent = state.persistent.lock().unwrap();
        persistent.completed_cycles = persistent.completed_cycles.saturating_add(1);
        let _ = persist_state(app, &persistent);
    }
    {
        let state = app.state::<AppState>();
        let settings = {
            let persistent = state.persistent.lock().unwrap();
            persistent.settings.clone()
        };
        let quote = {
            let persistent = state.persistent.lock().unwrap();
            current_quote(&persistent)
        };
        let mut runtime = state.runtime.lock().unwrap();
        runtime.phase = Phase::Break;
        runtime.remaining_seconds = settings.break_minutes * 60;
        runtime.total_seconds = runtime.remaining_seconds;
        runtime.paused = false;
        runtime.last_tick = Instant::now();
        runtime.current_quote = quote;
    }
    let _ = sync_lock_windows(app);
    emit_snapshot(app);
    tauri::async_runtime::spawn(refresh_online_quote(app.clone()));
}

fn exit_break(app: &AppHandle) {
    let (auto_restart, focus_min, break_min) = {
        let state = app.state::<AppState>();
        let persistent = state.persistent.lock().unwrap();
        (
            persistent.settings.auto_restart,
            persistent.settings.focus_minutes,
            persistent.settings.break_minutes,
        )
    };
    if auto_restart {
        close_lock_windows(app);
        if begin_focus(app, focus_min, break_min).is_err() {
            transition_to_idle(app);
        }
    } else {
        transition_to_idle(app);
    }
}

fn transition_to_idle(app: &AppHandle) {
    close_lock_windows(app);
    {
        let state = app.state::<AppState>();
        let mut runtime = state.runtime.lock().unwrap();
        runtime.phase = Phase::Idle;
        runtime.remaining_seconds = 0;
        runtime.total_seconds = 0;
        runtime.paused = false;
        runtime.last_tick = Instant::now();
    }
    emit_snapshot(app);
}

fn tick_runtime(app: &AppHandle) {
    enum Action {
        None,
        Emit,
        EnterBreak,
        ExitBreak,
    }

    let action = {
        let state = app.state::<AppState>();
        let mut runtime = state.runtime.lock().unwrap();

        if runtime.phase == Phase::Idle || runtime.paused {
            Action::None
        } else {
            let elapsed = runtime.last_tick.elapsed().as_secs();
            if elapsed == 0 {
                Action::None
            } else {
                runtime.last_tick = Instant::now();
                runtime.remaining_seconds = runtime.remaining_seconds.saturating_sub(elapsed);
                if runtime.remaining_seconds == 0 {
                    match runtime.phase {
                        Phase::Focus => Action::EnterBreak,
                        Phase::Break => Action::ExitBreak,
                        Phase::Idle => Action::None,
                    }
                } else {
                    Action::Emit
                }
            }
        }
    };

    match action {
        Action::None => {}
        Action::Emit => emit_snapshot(app),
        Action::EnterBreak => transition_to_break(app),
        Action::ExitBreak => exit_break(app),
    }
}

fn is_break_active(app: &AppHandle) -> bool {
    let state = app.state::<AppState>();
    let runtime = state.runtime.lock().unwrap();
    runtime.phase == Phase::Break
}

fn show_main_window(app: &AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.show();
        let _ = window.unminimize();
        let _ = window.set_focus();
    }
}

#[tauri::command]
fn get_snapshot(app: AppHandle) -> Snapshot {
    build_snapshot(&app)
}

#[tauri::command]
fn start_pomodoro(app: AppHandle, config: Option<StartConfig>) -> Result<Snapshot, String> {
    let state = app.state::<AppState>();
    let settings = {
        let persistent = state.persistent.lock().unwrap();
        persistent.settings.clone()
    };
    let cfg = config.unwrap_or(StartConfig {
        focus_minutes: settings.focus_minutes,
        break_minutes: settings.break_minutes,
    });
    begin_focus(&app, cfg.focus_minutes, cfg.break_minutes).map_err(|err| err.to_string())
}

#[tauri::command]
fn pause_pomodoro(app: AppHandle) -> Snapshot {
    {
        let state = app.state::<AppState>();
        let mut runtime = state.runtime.lock().unwrap();
        if runtime.phase == Phase::Focus {
            runtime.paused = true;
        }
    }
    emit_snapshot(&app);
    build_snapshot(&app)
}

#[tauri::command]
fn resume_pomodoro(app: AppHandle) -> Snapshot {
    {
        let state = app.state::<AppState>();
        let mut runtime = state.runtime.lock().unwrap();
        if runtime.phase == Phase::Focus {
            runtime.paused = false;
            runtime.last_tick = Instant::now();
        }
    }
    emit_snapshot(&app);
    build_snapshot(&app)
}

#[tauri::command]
fn cancel_pomodoro(app: AppHandle) -> Snapshot {
    transition_to_idle(&app);
    build_snapshot(&app)
}

#[tauri::command]
fn set_today_tasks(app: AppHandle, tasks: Vec<String>) -> Result<Vec<String>, String> {
    if tasks.len() > 3 {
        return Err("今日重要事项最多 3 件".to_string());
    }
    let mut slots: Vec<String> = tasks.into_iter().map(|t| t.trim().to_string()).collect();
    slots.resize(3, String::new());

    let state = app.state::<AppState>();
    {
        let mut persistent = state.persistent.lock().unwrap();
        ensure_today_fresh(&mut persistent);
        for (i, text) in slots.iter().enumerate() {
            if text.is_empty() {
                persistent.task_statuses[i] = "none".to_string();
            }
        }
        persistent.today_tasks = slots.clone();
        persist_state(&app, &persistent).map_err(|err| err.to_string())?;
    }
    emit_snapshot(&app);
    Ok(slots)
}

#[tauri::command]
fn set_task_status(app: AppHandle, index: usize, status: String) -> Snapshot {
    if index < 3 && matches!(status.as_str(), "none" | "doing" | "done") {
        let state = app.state::<AppState>();
        let mut persistent = state.persistent.lock().unwrap();
        ensure_today_fresh(&mut persistent);
        if !persistent.today_tasks[index].is_empty() {
            persistent.task_statuses[index] = status;
            let _ = persist_state(&app, &persistent);
        }
    }
    emit_snapshot(&app);
    build_snapshot(&app)
}

#[tauri::command]
fn get_settings(app: AppHandle) -> Settings {
    let state = app.state::<AppState>();
    let persistent = state.persistent.lock().unwrap();
    persistent.settings.clone()
}

#[tauri::command]
fn save_settings(app: AppHandle, settings: Settings) -> Result<Settings, String> {
    let saved = {
        let state = app.state::<AppState>();
        let mut persistent = state.persistent.lock().unwrap();
        persistent.settings.focus_minutes = settings.focus_minutes.clamp(1, 180);
        persistent.settings.break_minutes = settings.break_minutes.clamp(1, 60);
        persistent.settings.enable_online_quote = settings.enable_online_quote;
        persistent.settings.auto_restart = settings.auto_restart;
        persistent.settings.launch_on_login = settings.launch_on_login;
        persistent.settings.force_break = settings.force_break;
        persistent.settings.language = settings.language.clone();
        persist_state(&app, &persistent).map_err(|err| err.to_string())?;
        persistent.settings.clone()
    };
    apply_launch_at_login(saved.launch_on_login);
    emit_snapshot(&app);
    Ok(saved)
}

#[tauri::command]
fn exit_break_lock(app: AppHandle) -> Snapshot {
    exit_break(&app);
    build_snapshot(&app)
}

fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .manage(AppState::default())
        .setup(|app| {
            let app_handle = app.handle().clone();
            let persistent = load_persistent_state(&app_handle);
            let quote = current_quote(&persistent);

            {
                let state = app_handle.state::<AppState>();
                let mut persisted = state.persistent.lock().unwrap();
                *persisted = persistent.clone();
                let mut runtime = state.runtime.lock().unwrap();
                runtime.current_quote = quote;
            }

            let app_clone = app_handle.clone();
            thread::spawn(move || loop {
                thread::sleep(Duration::from_secs(1));
                tick_runtime(&app_clone);
            });

            let initial_menu = build_tray_menu(&app_handle, Phase::Idle, false, 0, true, &[], "")?;

            let tray_icon = tauri::image::Image::from_bytes(include_bytes!("../icons/tray.png"))?;

            let tray_builder = {
                let tray_app = app_handle.clone();
                TrayIconBuilder::with_id("main-tray")
                    .icon(tray_icon)
                    .menu(&initial_menu)
                    .tooltip("Focus Lock")
                    .on_menu_event(move |_app, event| {
                        match event.id.as_ref() {
                            "start" => {
                                let state = tray_app.state::<AppState>();
                                let (focus_min, break_min) = {
                                    let p = state.persistent.lock().unwrap();
                                    (p.settings.focus_minutes, p.settings.break_minutes)
                                };
                                let _ = begin_focus(&tray_app, focus_min, break_min);
                            }
                            "pause" => {
                                let state = tray_app.state::<AppState>();
                                let mut runtime = state.runtime.lock().unwrap();
                                if runtime.phase == Phase::Focus {
                                    runtime.paused = true;
                                    drop(runtime);
                                    emit_snapshot(&tray_app);
                                }
                            }
                            "resume" => {
                                let state = tray_app.state::<AppState>();
                                let mut runtime = state.runtime.lock().unwrap();
                                if runtime.phase == Phase::Focus {
                                    runtime.paused = false;
                                    runtime.last_tick = Instant::now();
                                    drop(runtime);
                                    emit_snapshot(&tray_app);
                                }
                            }
                            "cancel" => transition_to_idle(&tray_app),
                            "end_break" => exit_break(&tray_app),
                            id if id.starts_with("task_") => {
                                if let Ok(index) = id["task_".len()..].parse::<usize>() {
                                    let state = tray_app.state::<AppState>();
                                    let mut persistent = state.persistent.lock().unwrap();
                                    ensure_today_fresh(&mut persistent);
                                    if index < 3 && !persistent.today_tasks[index].is_empty() {
                                        let next = match persistent.task_statuses[index].as_str() {
                                            "none" => "doing",
                                            "doing" => "done",
                                            _ => "none",
                                        };
                                        persistent.task_statuses[index] = next.to_string();
                                        let _ = persist_state(&tray_app, &persistent);
                                    }
                                    drop(persistent);
                                    emit_snapshot(&tray_app);
                                }
                            }
                            "show" => show_main_window(&tray_app),
                            "about" => {
                                show_main_window(&tray_app);
                                let _ = tray_app.emit("open://about", ());
                            }
                            "quit" => {
                                close_lock_windows(&tray_app);
                                std::process::exit(0);
                            }
                            _ => {}
                        }
                    })
            };

            let tray_builder = {
                let tray_app2 = app_handle.clone();
                tray_builder.on_tray_icon_event(move |_tray, event| {
                    if matches!(
                        event,
                        TrayIconEvent::Click {
                            button: MouseButton::Left,
                            button_state: MouseButtonState::Up,
                            ..
                        }
                    ) {
                        show_main_window(&tray_app2);
                    }
                })
            };

            #[cfg(target_os = "macos")]
            let tray_builder = tray_builder.icon_as_template(true);

            tray_builder.build(app)?;

            tauri::async_runtime::spawn(refresh_online_quote(app_handle.clone()));
            Ok(())
        })
        .on_window_event(|window, event| {
            let app = window.app_handle();
            let label = window.label();
            match event {
                WindowEvent::CloseRequested { api, .. } => {
                    if label == "main" {
                        api.prevent_close();
                        let _ = window.hide();
                    } else if label.starts_with(LOCK_PREFIX) {
                        let state = app.state::<AppState>();
                        let runtime = state.runtime.lock().unwrap();
                        if runtime.phase == Phase::Break && !runtime.allow_lock_close {
                            api.prevent_close();
                        }
                    }
                }
                WindowEvent::Focused(false) => {
                    if label.starts_with(LOCK_PREFIX) && is_break_active(app) {
                        // On macOS the lock window stays visible at level 1000; only
                        // re-raise and re-focus via ObjC to avoid Tauri's
                        // set_always_on_top resetting the level to NSFloatingWindowLevel.
                        #[cfg(target_os = "macos")]
                        macos_relevel_lock(app, label);
                        #[cfg(not(target_os = "macos"))]
                        {
                            let _ = window.show();
                            let _ = window.set_always_on_top(true);
                            let _ = window.set_focus();
                        }
                    }
                }
                _ => {}
            }
        })
        .invoke_handler(tauri::generate_handler![
            get_snapshot,
            start_pomodoro,
            pause_pomodoro,
            resume_pomodoro,
            cancel_pomodoro,
            set_today_tasks,
            set_task_status,
            get_settings,
            save_settings,
            exit_break_lock,
        ])
        .build(tauri::generate_context!())
        .expect("failed to build tauri application")
        .run(|app, event| {
            if let RunEvent::Ready = event {
                let launch = {
                    let state = app.state::<AppState>();
                    let persistent = state.persistent.lock().unwrap();
                    persistent.settings.launch_on_login
                };
                apply_launch_at_login(launch);
                emit_snapshot(app);
            }
        });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn local_quotes_are_available() {
        assert!(local_quotes().len() >= 3);
        assert!(!local_quote_for_seed(1).text.is_empty());
    }

    #[test]
    fn stale_tasks_reset_when_date_changes() {
        let mut state = PersistentState {
            today_date: "2001-01-01".to_string(),
            today_tasks: vec!["A".to_string()],
            completed_cycles: 3,
            ..PersistentState::default()
        };
        ensure_today_fresh(&mut state);
        assert_eq!(state.today_date, today_string());
        assert!(state.today_tasks.iter().all(|t| t.is_empty()));
        assert_eq!(state.completed_cycles, 0);
    }
}
