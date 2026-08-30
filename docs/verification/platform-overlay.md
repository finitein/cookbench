# Platform Overlay Verification

## Contract

Cookbench manages only its own borderless Tauri windows. It does not control,
embed, foreground, or otherwise operate Codex, Claude Code, Pi, terminals, or
other harnesses. Basic macOS floating presentation does not request
Accessibility or Automation permission; a Cookbench-owned Windows topmost
window does not require elevation. On Ubuntu X11, Tauri requests the EWMH
keep-above behavior. GNOME Wayland remains a graphical fallback unless the
optional presentation-only GNOME extension is installed.

## Automated Evidence

| Evidence | Status | Recorded on |
| --- | --- | --- |
| Capability model: macOS full overlay | Passes in `platform_capabilities` | macOS development machine |
| Capability model: Windows full overlay without extension | Passes in `platform_capabilities` | macOS development machine |
| Capability model: Ubuntu X11 near-full overlay | Passes in `platform_capabilities` | macOS development machine |
| Capability model: GNOME Wayland without extension is graphical + best effort | Passes in `platform_capabilities` | macOS development machine |
| Capability model: GNOME Wayland with extension is full overlay | Passes in `platform_capabilities` | macOS development machine |
| Tauri compile and workspace tests | Passed: `cargo fmt --all -- --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace`, and `cargo build --workspace` | macOS development machine, 2026-08-30 |
| Native Ubuntu ARM64 compile and workspace tests | Passed: Rust fmt, strict Clippy, full workspace tests, 79 Vitest tests, TypeScript lint, and production frontend build | Ubuntu 24.04.4 GNOME X11 host, 2026-08-30 |
| Native Ubuntu packages | ARM64 DEB and AppImage built with executable bridge/hook sidecars; both package forms launched a Cookbench window | Ubuntu 24.04.4 GNOME X11 host, 2026-08-30 |

Automated tests prove the platform contract and compile the Tauri-backed window
operations. The native Ubuntu run additionally proves the tested X11 compositor
honored the keep-above request. Windows still requires a native runner.

## Manual Evidence Matrix

| Platform | OS version | Display scale(s) | Full-screen behavior | Multi-monitor behavior | Result | Evidence |
| --- | --- | --- | --- | --- | --- | --- |
| macOS Apple Silicon | macOS 26.3 (25D125), Apple M4 | 200% effective scale: 2560x1440 display appears as 1280x720 | Brand drag handle has capability, command, and persistence regression coverage; physical drag remains pending | Two detected displays are hardware mirrored; independent placement not yet manually exercised | Packaged 790x128 borderless Bar automatically populated from local native sessions, rendered the approved mark, and stayed above existing windows without an Accessibility prompt; full-screen and independent multi-monitor placement remain pending | [`evidence/macos-global-bar.png`](evidence/macos-global-bar.png), packaged app/DMG smoke, `sw_vers`, and `system_profiler SPDisplaysDataType`, 2026-08-30 |
| macOS Intel | Not available on this machine | Not verified | Not verified | Not verified | Not verified | VM or hardware required |
| Windows 10 | Not available on this machine | Not verified | Not verified | Not verified | Not verified | VM or hardware required |
| Windows 11 | Not available on this machine | Not verified | Not verified | Not verified | Not verified | VM or hardware required |
| Ubuntu 22.04 X11 | Not available on this machine | Not verified | Not verified | Not verified | Not verified | VM or hardware required |
| Ubuntu 24.04.4 GNOME X11, ARM64 | Linux 6.17.0-1014-nvidia on DGX Spark | 1920x1080, GNOME text scale 1.0 | Full-screen not exercised | Single connected display | Pass: native debug/release launch, DEB launch, AppImage launch, `_NET_WM_STATE_ABOVE`, and a compositor-applied move/resize to 820x300 at (120,140) | [`evidence/ubuntu-24.04-x11-global-bar.png`](evidence/ubuntu-24.04-x11-global-bar.png), package inventory, `wmctrl`, `xprop`, and `xwininfo`, 2026-08-30 |
| Ubuntu 24.04 GNOME Wayland, no extension | Not available on this machine | Not verified | Not verified | Not verified | Not verified; graphical fallback must be checked | VM or hardware required |
| Ubuntu 24.04 GNOME Wayland, extension | Not available on this machine | Not verified | Not verified | Not verified | Not verified | VM or hardware required |

The Ubuntu graphical smoke used an isolated temporary `HOME`, the logged-in
user's existing X11 session, and software rendering because a process launched
over SSH did not inherit direct DRM/GBM device access. The resulting process
stayed alive with one native window and an empty application log. No native
session content was read. AppImage validation used extract-and-run mode so the
result does not depend on FUSE being enabled on the verification host.

## Manual Checklist

- Record OS release, architecture, Tauri build version, and desktop session.
- Check one 100% scale display and one non-100% scale display where supported.
- Verify the borderless bright Cookbench Bar is legible with transparency reduced
  and that it does not pretend to be native macOS chrome on Windows or Ubuntu.
- Show the global Bar and a detached Bar; confirm both remain Cookbench-owned
  windows and no harness is manipulated.
- Move the global Bar to each connected monitor and record coordinates/scaling.
- Check behavior while another application is full-screen, then after sleep/wake.
- On macOS, verify basic presentation does not prompt for Accessibility.
- On Windows, verify normal-user operation does not prompt for elevation.
- On X11, inspect that the window manager honors the keep-above request.
- On GNOME Wayland without the extension, verify the bar remains usable even if
  the compositor declines topmost placement; record this as best effort.
- On GNOME Wayland with the extension, verify extension installation changes only
  presentation strength and never session discovery or harness control.
