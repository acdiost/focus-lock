# Focus Lock

Focus Lock is a Pomodoro timer with a forced break-time screen lock.

It is designed for people who want to avoid long uninterrupted work sessions and stop turning every break into more screen time. During focus sessions, it works like a normal timer. When a break starts, it covers every connected display with a full-screen rest screen that reminds you to step away, drink water, and move around.

![Focus Lock main window](focuslock-en.png)

## What It Helps You Do

- Set custom focus and break durations, such as 25 minutes of focus and 5 minutes of rest
- Lock all displays during breaks to reduce the urge to keep working
- Keep up to 3 important tasks for today and show them on the break screen
- See a quote and simple health reminders while resting
- Choose whether the next Pomodoro starts automatically after a break
- Choose whether breaks are forced and cannot be ended early
- Use the app in Chinese or English

## Download and Install

Download the package for your system from [Releases](https://github.com/acdiost/focus-lock/releases).

### macOS

- Apple Silicon Macs: download the `aarch64.dmg`
- Intel Macs: download the `x64.dmg`

Open the DMG and drag **FocusLock** into the **Applications** folder.

If macOS says the developer cannot be verified, right-click the app in Finder, choose **Open**, then confirm again.

If macOS says the app is damaged and cannot be opened, run:

```bash
xattr -cr /Applications/FocusLock.app
```

### Windows

Download the `.msi` installer and run it.

If Windows SmartScreen shows a warning, click **More info**, then **Run anyway**.

### Linux

Download either the AppImage or deb package.

AppImage:

```bash
chmod +x focus-lock_x.x.x_amd64.AppImage
./focus-lock_x.x.x_amd64.AppImage
```

deb:

```bash
sudo dpkg -i focus-lock_x.x.x_amd64.deb
```

## Basic Usage

1. Open Focus Lock.
2. Set your focus and break durations in the rhythm settings.
3. Add today's important tasks if you want them shown during breaks.
4. Click **Start Pomodoro** to begin focusing.
5. When the focus session ends, Focus Lock enters the break lock screen automatically.
6. When the break ends, return to the main window and start the next round.

If **Auto-start next round after break** is enabled, Focus Lock starts the next focus session automatically.

If **Force break** is enabled, the break cannot be ended early.

## Usage Tips

- Pick a focus duration you can actually sustain. Common choices are 25, 45, or 50 minutes.
- Set breaks to at least 5 minutes. Use longer breaks after longer work sessions.
- Keep today's important tasks short: 1 to 3 real priorities, not a full to-do list.
- When the break screen appears, try to leave the desk instead of switching to your phone.

## Notes

- Focus Lock is an app-level lock, not a system-level security lock. It helps you follow your break plan; it is not a security tool.
- Multi-display lock windows are created when a break starts. If you plug in a new monitor during a break, it may not be covered until the next break.
- macOS and Windows packages are currently unsigned, so the operating system may show a warning on first install.
- Linux desktop environments vary. Some Wayland window managers may not fully prevent workspace switching.

## Technical Documentation

Development, build, project structure, and implementation details are documented in [TECHNICAL-EN.md](TECHNICAL-EN.md).

## License

MIT © 2026 [acdiost](https://github.com/acdiost)
