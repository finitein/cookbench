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

On the macOS development runner, `pnpm test:e2e` passes all fourteen Chromium flows.
The test-only driver supplies sanitized snapshots, restart, detached placement,
clear, and outbound-notification observations in Vite `e2e` mode. A production
build scan confirms that the driver name, storage keys, and implementation are
absent from `dist`.

## v0.2.1 Release Candidate Gate

The 2026-08-31 macOS development run completed `./scripts/verify.sh` without
error: Rust formatting, workspace Clippy with warnings denied, all Rust unit and
integration tests, workspace build, TypeScript, 103 Vitest tests, three GNOME
protocol tests, fourteen product Playwright flows, production build, E2E-driver
absence scan, and source-only package inventory all passed.

The earlier `v0.2.0` tag did not publish a GitHub Release. Its package workflow
correctly stopped after the macOS bundle omitted architecture-specific sidecars
and the Linux/Windows audit treated generated system icon sizes as unrelated
runtime artwork. `v0.2.1` stages both per-architecture and universal helpers and
audits platform icon derivatives separately from the two approved runtime SVGs.
The repaired path produced a local universal macOS app and DMG containing
universal `cookbench-bridge` and `cookbench-hook` binaries; package smoke passed.

Additional release contracts passed:

- 27 unique Harness profiles, including the requested China-first tools, with
  tier, lifecycle, and return-surface invariants.
- Ten documentation, installer, manifest, and release-workflow contracts, plus
  the PNG renderer contract.
- Twelve showcase Playwright pages at exactly 1200x1500 with zero overflow and
  no remote assets; twelve PNG headers confirm 1200x1500 output.
- Visual verdict 94/100 after all twelve PNGs were inspected. Terminal states
  use complete rings and only numeric Cooking progress uses a partial arc.
- Fixture secret-pattern scan found no private key, provider token, live webhook,
  Telegram endpoint, or common cloud credential pattern.

## Acceptance Evidence

| # | Requirement | Evidence required | Current status |
| --- | --- | --- | --- |
| 1 | Graphical macOS, Windows, Ubuntu installation | Package smoke run on each platform | Local macOS universal app/DMG package smoke passes for arm64 and Intel binaries; Ubuntu 24.04 ARM64 DEB/AppImage and graphical package smoke previously passed; Windows and Ubuntu x64 Release CI pending |
| 2 | Codex, Claude Code, Pi create stoves from their original tools | Sanitized fixture E2E plus manual original-tool sessions | Three-harness native-parser and E2E coverage passes; packaged macOS app automatically discovered live local Codex session files; manual Claude Code and Pi native checks pending. The broader 27-profile catalog is Hook/manual/presence tiered and is not substituted for this native evidence |
| 3 | Every burner names its harness | `global-bar.spec.ts` plus screen-reader check | E2E and component accessible-name checks pass; manual screen reader pending |
| 4 | Global Bar contains all active and uncleared stoves | `global-bar.spec.ts` with three sources and retained Cooked fixture | E2E passes; packaged macOS snapshot populated from native sessions after the release ACL fallback regression was fixed |
| 5 | Hover/focus exposes safe project, task, state, activity, progress | Desktop screenshot and keyboard-focus check | Production summary projection, tooltip, and keyboard component tests pass; manual native hover screenshot pending |
| 6 | Needs Human and Cooked use authoritative transitions | State-machine and lifecycle evidence | Reducer, native Hook projection, cross-source ordering, and lifecycle E2E pass; live original-tool observation pending |
| 7 | Click returns to original surface or honest fallback | Locator tests and manual host checks | Twenty-two fallback tests pass, including Codex task deep links, exact terminal postconditions, unique generic-Harness correlation, ambiguity refusal, and presence-only refusal; live host checks remain pending |
| 8 | Cooked persists until explicit clear | `stove-lifecycle.spec.ts` restart and clear flow | E2E passes |
| 9 | Global and detached Bars coexist | `detached-bars.spec.ts` plus multi-window screenshot | Placement/restore/clear E2E passes; native multi-window screenshot pending |
| 10 | No transcript copy or harness control | Redaction tests, source audit, manual process inspection | Redaction, sanitized Hook projection, catalog, bridge protocol, fixture/source, and production-build audits pass; live original-tool process inspection pending |
| 11 | SSH disconnect never becomes Cooked | `notifications.spec.ts` and isolated SSH run | E2E plus thirteen zero-install SSH source tests and four bridge tests pass; isolated live remote remains pending |
| 12 | Outbound-only notifications filter by destination/state | `notifications.spec.ts`, mock endpoint audit, no listener/polling assertion | Mock sender and filtered E2E pass; live sandboxes pending |
| 13 | Attention, Cooked, Failed, Disconnected use complete rings | `stove-lifecycle.spec.ts`; screenshots | Component and E2E coverage pass; four browser screenshots recorded |
| 14 | Only structured Cooking has a determinate arc | `stove-lifecycle.spec.ts` with structured and empty progress fixtures | Component and E2E coverage pass |
| 15 | Approved light-default SVG/CSS material, no third-party logos/heavy media | Asset inventory and screenshot comparison with visual prototype | Product browser evidence and packaged macOS capture remain recorded; exact two-SVG master and package audits pass. Twelve new offline showcase compositions use only the Cookbench mark, system fonts, and CSS and passed a 94/100 visual verdict |
| 16 | Reduced motion and accessible state labels | Emulated media-feature screenshots and keyboard/screen-reader check | Reduced-motion E2E and component labels pass; OS/screen-reader check pending |
| 17 | No photos, GIFs, video, Lottie, sprites, or bundled web fonts | Package artifact inventory | Source-only, local macOS universal app/DMG, and Ubuntu ARM64 DEB/AppImage inventories pass; showcase remote/forbidden-asset checks pass; Windows and Ubuntu x64 package inventory pending Release CI |

## Visual Matrix

Capture the global Bar and one detached Bar against a light desktop background
and a dark desktop background. At every capture, inspect text clipping,
overlap, blank content, hidden stoves, and layout shifts.

| Viewport / scale | Global Bar | Detached Bar | Accessibility modes | Status |
| --- | --- | --- | --- | --- |
| 1280x720 at 100% | Browser evidence recorded | Native window pending | Default and reduced motion | E2E passed |
| 1440x900 at 100% | Dark-background evidence recorded | Native window pending | Reduced motion; increased contrast pending | E2E passed |
| 2560x1440 at 200% effective scale | Packaged macOS Bar recorded at 790x128 logical pixels | Native detached window pending | Reduced transparency pending | Global Bar passed; detached pending |
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
- Ubuntu X11: Ubuntu 24.04.4 ARM64 graphical launch, keep-above, arbitrary
  compositor move/resize, DEB, and AppImage are verified. Ubuntu 22.04,
  detached restore, and live notification delivery remain pending.
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
- Local alerts: verify sound-only defaults, explicit system-notification
  permission behavior, one-Stove flash, reduced motion, and desktop attention.
  Current native evidence and platform gaps are recorded in
  [`local-notifications.md`](local-notifications.md).

## Performance Evidence

The current macOS arm64 release process sampled at 92,944 KiB RSS and 0.0% CPU
after eight seconds, then 92,448 KiB and 0.0% after thirteen seconds. The hook
bounded spool self-test completed in 8 ms. See
`docs/verification/performance-macos.md` for method, structural scale checks,
and the native end-to-end latency gaps that remain open.

## Release Sign-Off

- [x] No unresolved critical E2E, security, or privacy issue.
- [x] Each row above has evidence or an explicit platform limitation.
- [x] No test fixture includes a real session, prompt, source code, command,
  credential, token, webhook URL, or SSH secret.
- [ ] Packaged artifacts pass the forbidden-asset inventory.
- [x] Release notes disclose unverified platforms and Wayland limitations.
