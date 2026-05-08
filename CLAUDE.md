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
- `app.js` — all client logic; calls Rust via `window.__TAURI__.invoke()` and subscribes to `pomodoro://state` events via `window.__TAURI__.event.listen()`

**Backend** (`src-tauri/src/main.rs`) — all Rust logic in a single file (~730 lines):
- `PersistentState` — serialized to JSON in the OS app config dir; holds settings, tasks, quote cache, completed cycle count; auto-resets daily state when the date changes
- `RuntimeState` — in-memory; holds current phase (`Idle`/`Focus`/`Break`), remaining seconds, pause flag, and current quote
- Background timer thread ticks every second, transitions phases, and emits `pomodoro://state` events to the frontend
- Break lock is implemented by creating a borderless full-screen `WebviewWindow` for each monitor at break start and destroying them at break end

**Tauri commands exposed to the frontend:**
`get_snapshot`, `start_pomodoro`, `pause_pomodoro`, `resume_pomodoro`, `cancel_pomodoro`, `get_today_tasks`, `set_today_tasks`, `get_settings`, `save_settings`, `enter_break_lock`, `exit_break_lock`

**Quotes:** Fetched from `hitokoto.cn` with a 24-hour cache in `PersistentState`; a hardcoded local fallback list is used when offline (`main.rs` line ~80).

**Tauri config** (`src-tauri/tauri.conf.json`): `allowlist.all: true` (all Tauri APIs enabled), no CSP, main window is 1180×820. Features `tray` and `custom-protocol` are required at build time (already in npm scripts).
