# Minimal Bar, Top Dock, and macOS Menu Bar Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Ship Cookbench v0.4.0 with a shared attention policy, Minimal Global Bar, cross-platform top docking, a macOS combined Stove status item, updated documentation and social assets, a verified GitHub release, and the released build installed locally.

**Architecture:** A pure `cookbench-core` selector produces one canonical priority order from existing Stove event metadata plus bounded Cooked acknowledgement cursors. React consumes that order for Full and Minimal Bar presentations, the native window layer owns monitor-safe top docking, and the existing macOS tray item renders an atomic variable-width Stove image with click hit testing. Existing lifecycle, adapters, privacy boundaries, notifications, locators, and Detached Stove behavior remain unchanged.

**Tech Stack:** Rust, Tauri 2, React 18, TypeScript, Vitest, Playwright, deterministic HTML/CSS showcase rendering, HyperFrames, GitHub Actions.

---

### Task 1: Lock and Implement the Shared Attention Policy

**Files:**
- Create: `crates/cookbench-core/src/presentation/attention.rs`
- Create: `crates/cookbench-core/src/presentation/mod.rs`
- Modify: `crates/cookbench-core/src/lib.rs`
- Modify: `crates/cookbench-core/src/persistence/state.rs`
- Modify: `crates/cookbench-core/src/persistence/mod.rs`
- Test: `crates/cookbench-core/tests/attention.rs`
- Test: `crates/cookbench-core/tests/persistence.rs`

**Steps:**
1. Write failing tests for the approved state ranking, newest-event tie break, deterministic identity tie break, acknowledged Cooked demotion, and a newer completion invalidating an older acknowledgement.
2. Run `cargo test -p cookbench-core --test attention --test persistence`; expect the new tests to fail because no presentation selector or acknowledgement cursor exists.
3. Add a pure selector that accepts bounded presentation inputs and returns ordered Stove identities without reading transcript content.
4. Add a bounded, serde-defaulted Cooked acknowledgement cursor keyed by Stove identity and completed-event metadata. Bump persisted state version only if the existing compatibility loader requires it, and retain old-version loading.
5. Run the focused tests; expect all new and existing core tests to pass.
6. Commit the core policy with a Lore message recording that priority is presentation-only and `pinned` is deliberately excluded.

### Task 2: Carry Priority and Acknowledgement Through the Desktop Store

**Files:**
- Modify: `src-tauri/src/app_state.rs`
- Modify: `src-tauri/src/events.rs`
- Modify: `src-tauri/src/commands/stoves.rs`
- Modify: `src-tauri/src/lib.rs`
- Modify: `src/types/stove.ts`
- Modify: `src/services/stoves.ts`
- Test: `src-tauri/tests/stove_events.rs`
- Test: `src-tauri/tests/desktop_persistence.rs`
- Test: `src/services/stoves.test.ts`

**Steps:**
1. Add failing Rust and TypeScript tests requiring each snapshot/change to carry one canonical `attentionOrder` and requiring activation of an unacknowledged Cooked Stove to persist acknowledgement without clearing it.
2. Run `cargo test -p cookbench-desktop --test stove_events --test desktop_persistence` and `pnpm test --run src/services/stoves.test.ts`; expect failures for missing fields/commands.
3. Compute the ordered IDs under the store lock from the shared selector and include them atomically with revisioned Stove changes.
4. Extend `StoveSync` to retain and validate the order, falling back to current insertion order only for a browser fixture that omits native data.
5. Add an idempotent acknowledgement command and call it from the existing activation flow only for Cooked Stoves.
6. Re-run the focused suites and commit.

### Task 3: Extend Display Settings and Backward-Compatible Persistence

**Files:**
- Modify: `crates/cookbench-core/src/persistence/config.rs`
- Modify: `src-tauri/src/commands/display.rs`
- Modify: `src/settings/display/service.ts`
- Modify: `src/settings/display/DisplaySettingsPanel.tsx`
- Modify: `src/hooks/useDisplaySettings.ts`
- Modify: `src/i18n/i18n.tsx`
- Modify: `src/settings/display/display-settings.css`
- Test: `crates/cookbench-core/tests/persistence.rs`
- Test: `src/settings/display/service.test.ts`
- Test: `src/settings/display/DisplaySettingsPanel.test.tsx`

**Steps:**
1. Write failing migration, validation, service, and UI tests for `globalBarMode` and `macStatusStoveCount`, including defaults `full` and `3`, clamping `0..8`, and hiding the count control off macOS.
2. Run the focused Rust and Vitest suites; expect missing-field failures.
3. Add serde-defaulted config fields and strict Rust command-boundary validation. Keep the existing display-settings event as the single live update channel.
4. Add an icon-based Full/Minimal segmented control and numeric stepper in Display Settings without changing unrelated settings.
5. Add all English, Simplified Chinese, Japanese, and Korean strings.
6. Re-run focused tests and commit.

### Task 4: Build the Minimal Global Bar

**Files:**
- Modify: `src/App.tsx`
- Modify: `src/components/GlobalBar.tsx`
- Modify: `src/components/StoveBurner.tsx`
- Modify: `src/components/benchLayout.ts`
- Create: `src/components/StovePriorityMenu.tsx`
- Modify: `src/components/global-bar.css`
- Modify: `src/styles/app.css`
- Test: `src/components/GlobalBar.test.tsx`
- Test: `src/components/benchLayout.test.ts`
- Test: `src/App.test.tsx`

**Steps:**
1. Write failing component tests proving Full mode still exposes every Stove, Minimal mode exposes exactly the first attention-ordered Stove, the complete list is reachable, and empty Minimal mode restores the Cookbench mark.
2. Run the focused Vitest files; expect missing-mode failures.
3. Add the persisted mode branch, Collapse/Expand icon controls, stable circular dimensions, right-click priority menu, safe tooltip, keyboard behavior, and reduced-motion handling by reusing existing Stove components.
4. Keep grouped Full mode intact and apply attention order within the existing layout contract.
5. Run focused tests and `pnpm lint`; inspect browser screenshots at desktop and narrow widths for overlap.
6. Run the visual-verdict loop against the approved Global Bar evidence until the score is at least 90, persisting the verdict under `.omx/state/global-bar/ralph-progress.json` without committing OMX state.
7. Commit the Minimal Bar.

### Task 5: Add Monitor-Safe Top Docking

**Files:**
- Create: `crates/cookbench-core/src/persistence/dock.rs`
- Modify: `crates/cookbench-core/src/persistence/config.rs`
- Modify: `src-tauri/src/commands/windows.rs`
- Modify: `src-tauri/src/commands/display.rs`
- Modify: `src-tauri/src/platform/overlay.rs`
- Modify: `src-tauri/src/platform/macos.rs`
- Modify: `src-tauri/src/platform/windows.rs`
- Modify: `src-tauri/src/platform/linux.rs`
- Modify: `src/services/globalBarWindow.ts`
- Modify: `src/hooks/useGlobalBarWindow.ts`
- Test: `src-tauri/tests/window_registry.rs`
- Test: `src-tauri/tests/platform_capabilities.rs`
- Test: `src/services/globalBarWindow.test.ts`

**Steps:**
1. Write failing pure-geometry and controller tests for the 12 px dock threshold, 24 px undock threshold, 3 px reveal strip, monitor fallback, Full/Minimal sizes, programmatic-move suppression, and Wayland visible fallback.
2. Run the focused Rust and Vitest suites; expect failures.
3. Add a persisted optional dock layout using existing monitor identity and relative horizontal positioning.
4. Add a native controller that evaluates user move completion, moves only within the selected monitor work area, collapses/expands safely, and never records its hidden coordinate as a free-form position.
5. Wire pointer entry, delayed pointer leave, focus, menu, drag, and resize intent through the existing Global Bar hook. Do not install a global mouse listener.
6. Re-run focused tests, verify multi-monitor math, and commit.

### Task 6: Build the macOS Combined Stove Status Item

**Files:**
- Create: `src-tauri/src/desktop_shell/status_stoves.rs`
- Modify: `src-tauri/src/desktop_shell/runtime.rs`
- Modify: `src-tauri/src/desktop_shell/controller.rs`
- Modify: `src-tauri/src/desktop_shell/mod.rs`
- Modify: `src-tauri/src/platform/macos.rs`
- Modify: `src-tauri/src/app_state.rs`
- Test: `src-tauri/tests/desktop_capabilities.rs`
- Test: `src-tauri/tests/stove_events.rs`
- Test: unit tests in `src-tauri/src/desktop_shell/status_stoves.rs`

**Steps:**
1. Write failing tests for RGBA output dimensions/nonblank pixels, zero/one/three/eight slots, stable slot assignment, exact boundary hit testing, stale-ID refusal, and static-icon fallback.
2. Run the focused Rust tests; expect the status manager to be missing.
3. Add a dependency-free rasterizer using Cookbench's state palette and simple ring/Harness marks. Keep rendering deterministic and bounded.
4. Reuse the existing tray item, disable automatic left-click menu display on macOS, map left-click X coordinates to the immutable slot snapshot, and keep right-click native menu behavior.
5. Rebuild the native menu from safe Stove presentation labels and existing shell commands whenever the snapshot changes.
6. Compile non-macOS targets structurally with platform gates, run focused tests, and commit.

### Task 7: Extend Browser and Native End-to-End Evidence

**Files:**
- Modify: `src/e2e/CookbenchE2EApp.tsx`
- Modify: `tests/e2e/fixtures.ts`
- Modify: `tests/e2e/global-bar.spec.ts`
- Create: `tests/e2e/minimal-dock.spec.ts`
- Modify: `tests/e2e/visual.spec.ts`
- Modify: `docs/verification/platform-overlay.md`
- Create: `docs/verification/minimal-dock-menu-bar.md`

**Steps:**
1. Add E2E driver support for priority order, Cooked acknowledgement, mode persistence, dock state, and platform capability snapshots without leaking real content.
2. Add flows for Full-to-Minimal switching, automatic priority replacement, complete-list access, restart restore, dock/reveal/undock, and reduced motion.
3. Run `pnpm test:e2e`; expect every product flow to pass.
4. Capture desktop and compact screenshots and run visual-verdict after each visual iteration until 90+.
5. Record honest native macOS/Windows/X11/Wayland evidence and explicit pending gaps; commit.

### Task 8: Update Product Documentation and Promotional Images

**Files:**
- Modify: `docs/plans/2026-08-29-cookbench-design.md`
- Modify: `README.md`
- Modify: `README.zh-CN.md`
- Modify: `README.ja.md`
- Modify: `README.ko.md`
- Modify: `docs/showcase/README.md`
- Modify: `docs/showcase/shared.css`
- Create: `docs/showcase/14-focus-surfaces.html`
- Modify: `scripts/render-showcase.mjs`
- Modify: showcase renderer tests under `tests/showcase/`
- Create: rendered 1200x1500 and channel-safe promotional PNGs under `docs/showcase/rendered/`

**Steps:**
1. Update the approved product contract so Full mode remains all-Stove while Minimal mode is the explicit priority exception.
2. Add concise feature sections and literal screenshots to all four README languages; keep installation and platform caveats aligned.
3. Add one new offline showcase composition presenting Minimal mode, top docking, and the macOS combined status item with no remote assets or invented claims.
4. Extend the deterministic renderer and its whitelist/contract tests, then run `pnpm showcase:render`.
5. Export a 1200x1500 Xiaohongshu carousel master and a 1080x1440 channel-safe variant. Inspect all output at original resolution.
6. Run visual-verdict against the existing Cookbench showcase style until 90+ and commit the source and approved PNGs.

### Task 9: Produce the Xiaohongshu and Douyin Feature Video

**Files:**
- Modify: `videos/cookbench-social-promos/STORYBOARD.md`
- Modify: `videos/cookbench-social-promos/SCRIPT.md`
- Modify: `videos/cookbench-social-promos/package.json`
- Create: `videos/cookbench-social-promos/compositions/04-focus-surfaces-vertical.html`
- Add: literal approved product captures under `videos/cookbench-social-promos/assets/showcase/`
- Create: `videos/cookbench-social-promos/renders/cookbench-04-focus-surfaces-vertical.mp4`
- Create: contact sheet under `videos/cookbench-social-promos/snapshots/`

**Steps:**
1. Load the HyperFrames product-launch, core, creative, animation, media, and CLI skills required by the existing project's `authoringSkill` and local contract.
2. Run `npx hyperframes@latest upgrade --project . --check` in the video project. Upgrade only if required, verify with `npx hyperframes check`, and record any version change.
3. Author one Chinese, silent-first 1080x1920 video of roughly 20-25 seconds: context-switching hook, Minimal mode, top docking, macOS menu bar, privacy boundary, and brand close. Include burned-in concise captions and preserve platform control safe zones.
4. Run the full HyperFrames check, render at high quality, and produce a contact sheet.
5. Inspect first/middle/last and transition frames. Run visual-verdict against existing promo contact sheets until 90+.
6. Verify with `ffprobe` that the result is 1080x1920, H.264-compatible MP4, correct duration, and has no unintended remote media.
7. Commit source, approved render, and contact sheet, excluding caches and transient TTS candidates.

### Task 10: Prepare v0.4.0

**Files:**
- Modify: `Cargo.toml`
- Modify: `Cargo.lock`
- Modify: `package.json`
- Modify: `src-tauri/tauri.conf.json`
- Modify: `packaging/release-matrix.json` if required by current contracts
- Modify: `README.md`
- Modify: `README.zh-CN.md`
- Modify: `README.ja.md`
- Modify: `README.ko.md`
- Modify: `docs/installing.md`
- Modify: `docs/verification/release-checklist.md`
- Create: `docs/releases/v0.4.0.md`

**Steps:**
1. Update every authoritative version source and current installation example from 0.3.0 to 0.4.0 while preserving historical release notes.
2. Write evidence-based release notes covering features, privacy invariants, platform caveats, promotion assets, installation, and known native gaps.
3. Run release contract tests and source-package audits; correct every stale version reference that is meant to track the current release.
4. Commit release preparation with a Lore message that names all untested native evidence honestly.

### Task 11: Verify, Package, and Update the Local Application

**Files:**
- Modify only files required to fix failures discovered by verification.
- Record evidence in `docs/verification/release-checklist.md` and the focused v0.4.0 verification document.

**Steps:**
1. Run `./scripts/verify.sh`; do not continue until formatting, Clippy, Rust tests/build, TypeScript, Vitest, GNOME protocol tests, Playwright, production isolation, and package audits all pass.
2. Inspect `git diff`, source inventory, generated artifacts, and fixture secret scans.
3. Build the local macOS application and DMG using the same universal configuration as Release CI when host toolchains permit it.
4. Install the locally built v0.4.0 application, launch it, verify its version and the three native features, then quit cleanly before CI release replacement.
5. Run a final independent code review and verification pass; fix and re-run the relevant gates.
6. Commit any verification-only corrections.

### Task 12: Publish GitHub v0.4.0 and Install the Released Build

**Files:**
- No source edits after the release commit except an evidence follow-up commit if the release checklist needs immutable workflow/artifact URLs.

**Steps:**
1. Ensure the release branch is clean except intentionally ignored local evidence and that every commit follows the Lore protocol.
2. Push `codex/minimal-dock-menu-bar-release`, integrate it into `main` without discarding unrelated remote work, and push `main`.
3. Create an annotated `v0.4.0` tag on the verified release commit and push it.
4. Monitor the GitHub Release workflow. Use the stable channel only when the configured signing gate passes; otherwise publish the repository's honest unsigned prerelease rather than misrepresenting signing.
5. Verify the public Release notes, manifest, SBOM, checksums, artifact inventory, and installer selection from downloaded public artifacts.
6. Install the released macOS v0.4.0 artifact on this machine, launch it, and confirm the installed/running application is v0.4.0.
7. Update release evidence with immutable URLs only if needed, run focused documentation contracts, commit/push that evidence without moving the already published tag, and report any gap explicitly.

## Completion Criteria

- The approved feature behavior works and is regression-protected.
- Full mode still exposes every visible Stove and all privacy/control invariants hold.
- Minimal mode, top docking, and the macOS status item have automated and honest native evidence.
- README and canonical docs describe v0.4.0 accurately in all maintained languages.
- The new Xiaohongshu images and Douyin-ready video are rendered, inspected, and reproducible.
- `./scripts/verify.sh` passes.
- GitHub contains the reviewed commits, v0.4.0 tag, and a complete Release with verified artifacts.
- The local installed Cookbench application is the released v0.4.0 build.
