# Local Notification Verification

Recorded on 2026-08-31. Local notification payloads contain only a bounded
Stove ID, project display label, and lifecycle event. They do not include
prompts, transcript text, commands, tool input, credentials, or native paths.

## Automated Evidence

| Check | Result |
| --- | --- |
| Legacy and fresh config default to sound only | Pass: core persistence migration and round-trip tests |
| Default lifecycle events are Needs Human, Cooked, Failed, and Disconnected | Pass: core persistence and desktop command tests |
| Immediate duplicate `(Stove, event)` alerts are suppressed | Pass: `local_notifications` integration test |
| Sound commands are fixed argv with no user-derived shell input | Pass: macOS, Windows, and Linux command-shape tests |
| Settings loads, saves, and tests all four channels | Pass: service and component tests |
| Feedback disappears after 20 seconds | Pass: fake-timer Settings test |
| Flash targets one global or detached Stove without changing layout | Pass: component tests and Playwright |
| Reduced motion removes the flash animation but keeps a static emphasis | Pass: CSS rule and reduced-motion E2E coverage |
| Full regression suite | Pass: Rust fmt, strict workspace Clippy, workspace tests, 98 Vitest tests, 13 Playwright tests, TypeScript lint, and production frontend build |

## Native Platform Evidence

| Platform | Result | Honest limitation |
| --- | --- | --- |
| macOS 26.3, arm64 | Packaged `/Applications/Cookbench.app` launched. Native Settings exposed sound enabled and the other three channels disabled. Explicit Sound was accepted into the bounded delivery worker; System notification, Flash Stove, and Request attention tests returned delivered. The installed bundle passed strict deep ad-hoc signature verification. | The automation verified Cookbench's native command result and UI state; it did not independently measure audio output, inspect Notification Center history, or measure Dock animation. |
| Ubuntu 24.04.4 GNOME X11, arm64 | The logged-in graphical session's D-Bus notification path accepted `notify-send`, and `canberra-gtk-play --id message` exited successfully when run with the session's real `DISPLAY=:1` and user D-Bus address. | The updated Cookbench binary was not compiled or launched on this host because it currently has no Rust or pnpm toolchain. Tauri banner, Stove flash, and urgency delivery therefore remain unverified in the updated package. |
| Windows 10/11 | Command-shape tests cover the fixed, noninteractive system-sound invocation; Tauri compilation on macOS remains green. | No native Windows runner was available, so toast delivery, taskbar attention, installed-app identity, and audible output remain unverified. |

System banners use the official Tauri 2 notification plugin. Cookbench requests
permission only after an explicit enable or test action; accepted Session
transitions never open a permission prompt. Channel failures are best effort
and never change a Stove or Agent state.
