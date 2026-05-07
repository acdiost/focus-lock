#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::{
    collections::HashSet,
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
    api::path::app_config_dir, AppHandle, CustomMenuItem, GlobalWindowEvent, Icon, Manager,
    RunEvent, SystemTray, SystemTrayEvent, SystemTrayMenu, WindowBuilder, WindowEvent, WindowUrl,
};

const STATE_FILE_NAME: &str = "state.json";
const LOCK_PREFIX: &str = "lock-screen-";
const ONLINE_QUOTE_ENDPOINT: &str = "https://v1.hitokoto.cn/?encode=json";

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Settings {
    focus_minutes: u64,
    break_minutes: u64,
    enable_online_quote: bool,
    enable_water_reminder: bool,
    enable_stand_reminder: bool,
    launch_on_login: bool,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            focus_minutes: 25,
            break_minutes: 5,
            enable_online_quote: true,
            enable_water_reminder: true,
            enable_stand_reminder: true,
            launch_on_login: false,
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
    quote_cache: Option<Quote>,
    completed_cycles: u32,
}

impl Default for PersistentState {
    fn default() -> Self {
        Self {
            settings: Settings::default(),
            today_date: today_string(),
            today_tasks: vec![],
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
}

fn today_string() -> String {
    Local::now().format("%Y-%m-%d").to_string()
}

fn state_file_path(app: &AppHandle) -> PathBuf {
    let mut root = app_config_dir(&app.config()).unwrap_or_else(|| PathBuf::from("."));
    root.push("Focus Lock");
    root.push(STATE_FILE_NAME);
    root
}

fn ensure_today_fresh(persistent: &mut PersistentState) {
    let today = today_string();
    if persistent.today_date != today {
        persistent.today_date = today;
        persistent.today_tasks.clear();
        persistent.completed_cycles = 0;
    }
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
    Local::now().timestamp().unsigned_abs() as usize
}

fn local_quotes() -> &'static [(&'static str, &'static str)] {
    &[
        ("先完成最重要的一件事，剩下的噪音会自己退后。", "Focus Lock"),
        ("专注不是压榨自己，而是替注意力清场。", "Focus Lock"),
        (
            "屏幕之外的几分钟休息，会换回来更长的稳定输出。",
            "Focus Lock",
        ),
        ("不要把休息当中断，它是下一段专注的准备动作。", "Focus Lock"),
        ("今天真正重要的事情，通常不会超过三件。", "Focus Lock"),
        ("把目光从像素里拔出来，身体才会跟上你的节奏。", "Focus Lock"),
        (
            "好的节奏不是一直冲刺，而是知道什么时候停下来。",
            "Focus Lock",
        ),
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
        quote: runtime.current_quote.clone(),
    }
}

fn emit_snapshot(app: &AppHandle) {
    let snapshot = build_snapshot(app);
    let _ = app.emit_all("pomodoro://state", snapshot);
}

fn current_quote(persistent: &PersistentState) -> Quote {
    persistent
        .quote_cache
        .clone()
        .unwrap_or_else(|| local_quote_for_seed(local_quote_seed()))
}

fn close_lock_windows(app: &AppHandle) {
    {
        let state = app.state::<AppState>();
        let mut runtime = state.runtime.lock().unwrap();
        runtime.allow_lock_close = true;
    }

    let labels: Vec<String> = app
        .windows()
        .keys()
        .filter(|label| label.starts_with(LOCK_PREFIX))
        .cloned()
        .collect();

    for label in labels {
        if let Some(window) = app.get_window(&label) {
            let _ = window.close();
        }
    }

    let state = app.state::<AppState>();
    let mut runtime = state.runtime.lock().unwrap();
    runtime.allow_lock_close = false;
}

fn sync_lock_windows(app: &AppHandle) -> tauri::Result<()> {
    close_lock_windows(app);

    let Some(main_window) = app.get_window("main") else {
        return Ok(());
    };
    let monitors = main_window.available_monitors()?;
    for (index, monitor) in monitors.into_iter().enumerate() {
        let label = format!("{LOCK_PREFIX}{index}");
        let size = monitor.size();
        let position = monitor.position();
        let window =
            WindowBuilder::new(app, label.clone(), WindowUrl::App("index.html#lock".into()))
                .decorations(false)
                .always_on_top(true)
                .resizable(false)
                .skip_taskbar(true)
                .visible(false)
                .title("Focus Lock Break")
                .inner_size(size.width as f64, size.height as f64)
                .position(position.x as f64, position.y as f64)
                .build()?;

        let _ = window.set_always_on_top(true);
        let _ = window.set_focus();
        let _ = window.show();
    }

    Ok(())
}

async fn refresh_online_quote(app: AppHandle) {
    let should_fetch = {
        let state = app.state::<AppState>();
        let persistent = state.persistent.lock().unwrap();
        persistent.settings.enable_online_quote
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
    let snapshot = build_snapshot(app);
    let _ = app.emit_all("pomodoro://state", snapshot.clone());
    Ok(snapshot)
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
        SyncLocks,
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
                } else if runtime.phase == Phase::Break && elapsed > 0 {
                    Action::SyncLocks
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
        Action::ExitBreak => transition_to_idle(app),
        Action::SyncLocks => {
            let _ = sync_lock_windows(app);
            emit_snapshot(app);
        }
    }
}

fn focus_lock_window_labels(app: &AppHandle) -> HashSet<String> {
    app.windows()
        .keys()
        .filter(|label| label.starts_with(LOCK_PREFIX))
        .cloned()
        .collect()
}

fn is_break_active(app: &AppHandle) -> bool {
    let state = app.state::<AppState>();
    let runtime = state.runtime.lock().unwrap();
    runtime.phase == Phase::Break
}

fn show_main_window(app: &AppHandle) {
    if let Some(window) = app.get_window("main") {
        let _ = window.show();
        let _ = window.unminimize();
        let _ = window.set_focus();
    }
}

fn tray_icon() -> Icon {
    const SIZE: u32 = 18;
    let mut rgba = vec![0_u8; (SIZE * SIZE * 4) as usize];

    for y in 0..SIZE {
        for x in 0..SIZE {
            let idx = ((y * SIZE + x) * 4) as usize;
            let is_bar = (4..=13).contains(&x) && (6..=8).contains(&y);
            let is_stem = (8..=10).contains(&x) && (9..=14).contains(&y);
            let is_base = (5..=13).contains(&x) && (15..=16).contains(&y);
            let is_pixel_on = is_bar || is_stem || is_base;
            rgba[idx] = 0;
            rgba[idx + 1] = 0;
            rgba[idx + 2] = 0;
            rgba[idx + 3] = if is_pixel_on { 255 } else { 0 };
        }
    }

    Icon::Rgba {
        rgba,
        width: SIZE,
        height: SIZE,
    }
}

fn handle_lock_window_event(event: GlobalWindowEvent) {
    let window = event.window();
    let app = window.app_handle();
    let lock_labels = focus_lock_window_labels(&app);
    if !lock_labels.contains(window.label()) {
        return;
    }

    match event.event() {
        WindowEvent::CloseRequested { api, .. } => {
            let state = app.state::<AppState>();
            let runtime = state.runtime.lock().unwrap();
            if runtime.phase == Phase::Break && !runtime.allow_lock_close {
                api.prevent_close();
            }
        }
        WindowEvent::Focused(false) => {
            if is_break_active(&app) {
                let _ = window.show();
                let _ = window.set_always_on_top(true);
                let _ = window.set_focus();
            }
        }
        _ => {}
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
        if runtime.phase != Phase::Idle {
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
        if runtime.phase != Phase::Idle {
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
fn get_today_tasks(app: AppHandle) -> Vec<String> {
    let state = app.state::<AppState>();
    let mut persistent = state.persistent.lock().unwrap();
    ensure_today_fresh(&mut persistent);
    persistent.today_tasks.clone()
}

#[tauri::command]
fn set_today_tasks(app: AppHandle, tasks: Vec<String>) -> Result<Vec<String>, String> {
    let mut cleaned = Vec::new();
    for task in tasks.into_iter().map(|task| task.trim().to_string()) {
        if !task.is_empty() {
            cleaned.push(task);
        }
    }
    if cleaned.len() > 3 {
        return Err("今日重要事项最多 3 件".to_string());
    }

    let state = app.state::<AppState>();
    {
        let mut persistent = state.persistent.lock().unwrap();
        ensure_today_fresh(&mut persistent);
        persistent.today_tasks = cleaned.clone();
        persist_state(&app, &persistent).map_err(|err| err.to_string())?;
    }
    emit_snapshot(&app);
    Ok(cleaned)
}

#[tauri::command]
fn get_settings(app: AppHandle) -> Settings {
    let state = app.state::<AppState>();
    let persistent = state.persistent.lock().unwrap();
    persistent.settings.clone()
}

#[tauri::command]
fn save_settings(app: AppHandle, settings: Settings) -> Result<Settings, String> {
    {
        let state = app.state::<AppState>();
        let mut persistent = state.persistent.lock().unwrap();
        persistent.settings.focus_minutes = settings.focus_minutes.clamp(1, 180);
        persistent.settings.break_minutes = settings.break_minutes.clamp(1, 60);
        persistent.settings.enable_online_quote = settings.enable_online_quote;
        persistent.settings.enable_water_reminder = settings.enable_water_reminder;
        persistent.settings.enable_stand_reminder = settings.enable_stand_reminder;
        persistent.settings.launch_on_login = settings.launch_on_login;
        persist_state(&app, &persistent).map_err(|err| err.to_string())?;
    }
    emit_snapshot(&app);
    Ok(settings)
}

#[tauri::command]
fn enter_break_lock(app: AppHandle) -> Snapshot {
    transition_to_break(&app);
    build_snapshot(&app)
}

#[tauri::command]
fn exit_break_lock(app: AppHandle) -> Snapshot {
    transition_to_idle(&app);
    build_snapshot(&app)
}

fn main() {
    tauri::Builder::default()
        .manage(AppState::default())
        .setup(|app| {
            let app_handle = app.handle();
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

            let tray_app = app_handle.clone();
            let mut tray = SystemTray::new()
                .with_icon(tray_icon())
                .with_menu(
                    SystemTrayMenu::new()
                        .add_item(CustomMenuItem::new("show".to_string(), "显示主窗口"))
                        .add_item(CustomMenuItem::new("quit".to_string(), "退出")),
                )
                .with_tooltip("Focus Lock")
                .on_event(move |event| match event {
                    SystemTrayEvent::LeftClick { .. } => show_main_window(&tray_app),
                    SystemTrayEvent::MenuItemClick { id, .. } => match id.as_str() {
                        "show" => show_main_window(&tray_app),
                        "quit" => {
                            close_lock_windows(&tray_app);
                            std::process::exit(0);
                        }
                        _ => {}
                    },
                    _ => {}
                });

            #[cfg(target_os = "macos")]
            {
                tray = tray.with_icon_as_template(true);
            }

            tray.build(app)?;
            tauri::async_runtime::spawn(refresh_online_quote(app_handle.clone()));
            Ok(())
        })
        .on_window_event(handle_lock_window_event)
        .invoke_handler(tauri::generate_handler![
            get_snapshot,
            start_pomodoro,
            pause_pomodoro,
            resume_pomodoro,
            cancel_pomodoro,
            get_today_tasks,
            set_today_tasks,
            get_settings,
            save_settings,
            enter_break_lock,
            exit_break_lock,
        ])
        .build(tauri::generate_context!())
        .expect("failed to build tauri application")
        .run(|app, event| {
            if let RunEvent::Ready = event {
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
        assert!(state.today_tasks.is_empty());
        assert_eq!(state.completed_cycles, 0);
    }
}
