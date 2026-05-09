# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Commands

```bash
# Development (hot-reload)
npm run dev

# Production build
npm run build

# Rust unit tests
cd src-tauri && cargo test
```

The frontend (`src/`) is served directly with no build step — changes to HTML/CSS/JS are reflected immediately in dev mode.

## Architecture

Focus Lock is a Tauri desktop app: a Pomodoro timer that locks all displays during breaks by covering them with full-screen windows showing tasks, quotes, and reminders.

**Frontend** (`src/`) — plain JS (ES modules), no framework, no TypeScript:
- `index.html` — single page with two views: main timer UI and break lock screen
- `app.js` — all client logic; calls Rust via `window.__TAURI__.core.invoke()` and subscribes to `pomodoro://state` events via `window.__TAURI__.event.listen()`

**Backend** (`src-tauri/src/main.rs`) — all Rust logic in a single file (~1100 lines):
- `PersistentState` — serialized to JSON in the OS app config dir; holds settings, tasks, quote cache, completed cycle count; auto-resets daily state when the date changes
- `RuntimeState` — in-memory; holds current phase (`Idle`/`Focus`/`Break`), remaining seconds, pause flag, and current quote
- Background timer thread ticks every second, transitions phases, and emits `pomodoro://state` events to the frontend
- Break lock is implemented by creating a borderless full-screen `WebviewWindowBuilder` for each monitor at break start and destroying them at break end
- System tray built with `TrayIconBuilder::with_id("main-tray")` and `tauri::menu::Menu`; menu is rebuilt on every state change via `app.tray_by_id("main-tray")`

**Tauri commands exposed to the frontend:**
`get_snapshot`, `start_pomodoro`, `pause_pomodoro`, `resume_pomodoro`, `cancel_pomodoro`, `set_today_tasks`, `set_task_status`, `get_settings`, `save_settings`, `exit_break_lock`

**Quotes:** Fetched from `hitokoto.cn` with a 24-hour cache in `PersistentState`; a hardcoded local fallback list is used when offline (`main.rs` line ~215).

**Tauri config** (`src-tauri/tauri.conf.json`): Tauri v2 format — no `allowlist`, permissions are defined in `src-tauri/capabilities/default.json` (`core:default` + `shell:allow-open`), no CSP, main window is 1180×820. Tray icon feature is enabled via `Cargo.toml` (`tray-icon`, `image-png`), no CLI feature flags needed.
