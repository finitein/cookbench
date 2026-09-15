# Windows Settings webview smoke

## Bug

Secondary Settings `WebviewWindow` opened as a titled shell whose CDP URL stayed
`about:blank` (white screen) on Windows while the main Global Bar loaded
`http://tauri.localhost/`. Linux AppImage was fine.

## Cause

`open_notification_settings` built the window from a **synchronous** Tauri
command, and the tray Open Settings path called the same builder from a
**menu/event handler**. On Windows, WebView2 deadlocks in that context
(documented on `WebviewWindowBuilder::new`), leaving the shell blank.

## Fix

- Async `open_notification_settings` command.
- Tray uses `open_settings_window_off_thread` (`std::thread::spawn`).
- Reopen destroys a leftover `about:blank` shell before recreating with
  `WebviewUrl::App("index.html")` (same entry as the main window).

## Verify on Windows (leau7600x or equivalent)

1. Build or install a package that includes the fix commit.
2. Launch Cookbench; empty Bar CTA or gear / tray → Open Settings.
3. Confirm Settings title is localized and content shows Local Sources / tabs
   (not a white page). Optional: DevTools/CDP URL is `http://tauri.localhost/`
   (or `https://tauri.localhost/`), not `about:blank`.
4. Close Settings, open again via empty-Bar CTA → should land on Local Sources
   (deep-link via `settingsTab.ts`).
5. If a blank Settings window was left from an older build in the same process,
   reopen should recreate and load the frontend.

Unit regression: `cargo test --lib settings_window_tests` in `src-tauri`.

## Smoke result (2026-09-16, leau7600x)

- Branch commit: `1d96600`
- Built: `pnpm tauri build --no-bundle` → `target/release/cookbench-desktop.exe`
- Launched with `WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS=--remote-debugging-port=9333`
- CDP invoke `open_notification_settings` from main → second page at `http://tauri.localhost/`
- Settings DOM showed tabs including **本地来源** (not white/`about:blank`)
- Status: **PASS**
