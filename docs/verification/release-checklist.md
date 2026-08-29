# Cookbench Release Checklist

This checklist records evidence, not assumptions. A checked item must link to
an automated result, screenshot, or manual run record. Platform entries that
cannot be run on the release host remain pending.

## Required Automated Commands

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
pnpm test --run
pnpm build
pnpm test:e2e
```

On the macOS development runner, `pnpm test:e2e` passes all nine Chromium flows.
The test-only driver supplies sanitized snapshots, restart, detached placement,
clear, and outbound-notification observations in Vite `e2e` mode. A production
build scan confirms that the driver name, storage keys, and implementation are
absent from `dist`.

## Acceptance Evidence

| # | Requirement | Evidence required | Current status |
| --- | --- | --- | --- |
| 1 | Graphical macOS, Windows, Ubuntu installation | Package smoke run on each platform | macOS arm64 app/DMG build and package smoke pass; Windows, Ubuntu, and macOS Intel pending |
| 2 | Codex, Claude Code, Pi create stoves from their original tools | Sanitized fixture E2E plus manual original-tool sessions | Three-harness E2E passes; manual original-tool sessions pending |
| 3 | Every burner names its harness | `global-bar.spec.ts` plus screen-reader check | E2E and component accessible-name checks pass; manual screen reader pending |
| 4 | Global Bar contains all active and uncleared stoves | `global-bar.spec.ts` with three sources and retained Cooked fixture | E2E passes |
| 5 | Hover/focus exposes safe project, task, state, activity, progress | Desktop screenshot and keyboard-focus check | Production summary projection, tooltip, and keyboard component tests pass; manual native hover screenshot pending |
| 6 | Needs Human and Cooked use authoritative transitions | State-machine and lifecycle evidence | Reducer, native Hook projection, cross-source ordering, and lifecycle E2E pass; live original-tool observation pending |
| 7 | Click returns to original surface or honest fallback | Locator tests and manual host checks | Automated fallback coverage; host checks pending |
| 8 | Cooked persists until explicit clear | `stove-lifecycle.spec.ts` restart and clear flow | E2E passes |
| 9 | Global and detached Bars coexist | `detached-bars.spec.ts` plus multi-window screenshot | Placement/restore/clear E2E passes; native multi-window screenshot pending |
| 10 | No transcript copy or harness control | Redaction tests, source audit, manual process inspection | Redaction and fixture/source audits pass; live original-tool process inspection pending |
| 11 | SSH disconnect never becomes Cooked | `notifications.spec.ts` and isolated SSH run | E2E passes; isolated live remote pending |
| 12 | Outbound-only notifications filter by destination/state | `notifications.spec.ts`, mock endpoint audit, no listener/polling assertion | Mock sender and filtered E2E pass; live sandboxes pending |
| 13 | Attention, Cooked, Failed, Disconnected use complete rings | `stove-lifecycle.spec.ts`; screenshots | Component and E2E coverage pass; four browser screenshots recorded |
| 14 | Only structured Cooking has a determinate arc | `stove-lifecycle.spec.ts` with structured and empty progress fixtures | Component and E2E coverage pass |
| 15 | Approved light-default SVG/CSS material, no third-party logos/heavy media | Asset inventory and screenshot comparison with visual prototype | Four browser screenshots recorded; exact two-SVG master comparison and macOS package audit pass |
| 16 | Reduced motion and accessible state labels | Emulated media-feature screenshots and keyboard/screen-reader check | Reduced-motion E2E and component labels pass; OS/screen-reader check pending |
| 17 | No photos, GIFs, video, Lottie, sprites, or bundled web fonts | Package artifact inventory | macOS arm64 app/DMG package inventory passes; Windows and Ubuntu artifacts pending |

## Visual Matrix

Capture the global Bar and one detached Bar against a light desktop background
and a dark desktop background. At every capture, inspect text clipping,
overlap, blank content, hidden stoves, and layout shifts.

| Viewport / scale | Global Bar | Detached Bar | Accessibility modes | Status |
| --- | --- | --- | --- | --- |
| 1280x720 at 100% | Browser evidence recorded | Native window pending | Default and reduced motion | E2E passed |
| 1440x900 at 100% | Dark-background evidence recorded | Native window pending | Reduced motion; increased contrast pending | E2E passed |
| 2560x1440 at 200% effective scale | Existing macOS window evidence | Native window pending | Reduced transparency pending | Partial macOS evidence |
| 390x844 at 100% browser viewport | Responsive evidence recorded | Not applicable | Keyboard focus covered by components | E2E passed |

Compare each capture with `docs/visual-prototype/index.html`: bright
light-default material, approved Cookbench mark, source text tokens, vivid
state colors, full terminal/attention rings, and no runtime raster or motion
asset additions. Verify blur-unavailable fallback is solid and legible.

## Platform and Remote Checks

- macOS: normal-user overlay, detached window, display scale, full-screen,
  reduced transparency, notification permission denial, and original-terminal
  fallback. Do not claim Accessibility permission is required for basic UI.
- Windows: normal-user topmost window, detached restore, 100% and 150% scale,
  foreground/elevated-target fallback, and outbound notification send.
- Ubuntu 22.04 X11: graphical app, keep-above behavior, detached restore, and
  notification send.
- Ubuntu 24.04 GNOME Wayland: graphical fallback without extension, then
  presentation-only extension behavior. Record best-effort limitations rather
  than treating them as failures or full-overlay success.
- SSH: test zero-install read-only discovery and optional bridge separately;
  disconnect must remain Disconnected, no Cooked notification may be emitted,
  bridge must use SSH stdio only and open no port. Automated checks cover
  configured-root transport, project metadata projection, checksum enforcement,
  fixed OpenSSH liveness deadlines, and local stalled-child termination; an
  isolated live host remains pending.
- Notifications: use synthetic sandbox destinations for Telegram, Slack,
  Discord, Lark/Feishu, and Generic Webhook. Confirm no inbound listener,
  polling, response processing, or remote agent control is present.

## Performance Evidence

The current macOS arm64 release process sampled at 92,944 KiB RSS and 0.0% CPU
after eight seconds, then 92,448 KiB and 0.0% after thirteen seconds. The hook
bounded spool self-test completed in 8 ms. See
`docs/verification/performance-macos.md` for method, structural scale checks,
and the native end-to-end latency gaps that remain open.

## Release Sign-Off

- [ ] No unresolved critical E2E, security, or privacy issue.
- [ ] Each row above has evidence or an explicit platform limitation.
- [ ] No test fixture includes a real session, prompt, source code, command,
  credential, token, webhook URL, or SSH secret.
- [ ] Packaged artifacts pass the forbidden-asset inventory.
- [ ] Release notes disclose unverified platforms and Wayland limitations.
