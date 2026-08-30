# Cookbench Local Notifications Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Add configurable local sound, system banner, Stove flash, and desktop attention notifications, with sound as the only default channel.

**Architecture:** Persist additive local alert preferences in the existing config, then deliver alerts from the accepted-transition boundary in `AppState`. Native effects stay behind a small Rust runtime; React handles only the metadata-only Stove flash and Settings controls.

**Tech Stack:** Rust, serde, Tauri 2, `tauri-plugin-notification`, React, TypeScript, Vitest, Playwright, CSS.

---

### Task 1: Persist Local Alert Preferences

**Files:**
- Modify: `crates/cookbench-core/src/persistence/config.rs`
- Modify: `crates/cookbench-core/tests/persistence.rs`

**Step 1: Write the failing tests**

Add tests proving a missing local-alert field defaults to sound on, other channels off, and the four approved default events. Add a round-trip test for a custom channel and event selection.

**Step 2: Run the focused tests and verify RED**

Run: `PATH=/opt/homebrew/opt/rustup/bin:$PATH cargo test -p cookbench-core --test persistence local_notification`

Expected: FAIL because the preference type and field do not exist.

**Step 3: Implement the schema**

Add a serde-defaulted `LocalNotificationPreferences` to `UserPreferences`. Reuse `NotificationEventKind`, bound and de-duplicate configured events, and preserve the legacy `notifications_enabled` field for migration compatibility.

**Step 4: Run focused and package verification**

Run:

```bash
PATH=/opt/homebrew/opt/rustup/bin:$PATH cargo test -p cookbench-core --test persistence
PATH=/opt/homebrew/opt/rustup/bin:$PATH cargo clippy -p cookbench-core --all-targets -- -D warnings
```

Expected: PASS.

**Step 5: Commit using the Lore protocol**

Commit only the schema and persistence tests.

### Task 2: Deliver Native Local Alerts

**Files:**
- Modify: `src-tauri/Cargo.toml`
- Modify: `Cargo.lock`
- Replace: `src-tauri/src/notifications/local.rs`
- Modify: `src-tauri/src/notifications/mod.rs`
- Create: `src-tauri/tests/local_notifications.rs`

**Step 1: Write failing runtime tests**

Cover event filtering, duplicate suppression, sanitized messages, fixed sound-driver arguments, bounded failures, and permission-denied banner behavior with fake backends.

**Step 2: Run focused tests and verify RED**

Run: `PATH=/opt/homebrew/opt/rustup/bin:$PATH cargo test -p cookbench-desktop --test local_notifications`

Expected: FAIL because the production local runtime does not exist.

**Step 3: Implement the runtime**

Build a channel-independent dispatcher. Use fixed, argv-only platform sound commands, Tauri window attention, metadata-only flash emission, and the official Tauri notification plugin for banners. Permission requests are exposed only through explicit Settings commands.

**Step 4: Verify the runtime**

Run:

```bash
PATH=/opt/homebrew/opt/rustup/bin:$PATH cargo test -p cookbench-desktop --test local_notifications
PATH=/opt/homebrew/opt/rustup/bin:$PATH cargo clippy -p cookbench-desktop --all-targets -- -D warnings
```

Expected: PASS.

**Step 5: Commit using the Lore protocol**

Commit the native runtime, dependency, and focused tests.

### Task 3: Wire Commands And Accepted Transitions

**Files:**
- Modify: `src-tauri/src/commands/notifications.rs`
- Modify: `src-tauri/src/app_state.rs`
- Modify: `src-tauri/src/lib.rs`
- Modify: `src-tauri/tests/stove_events.rs`
- Create: `src-tauri/tests/local_notification_commands.rs`

**Step 1: Write failing integration tests**

Prove Settings reads/saves defaults, explicit tests address one channel, accepted transitions alert, and stale/superseded transitions remain inert.

**Step 2: Run tests and verify RED**

Run: `PATH=/opt/homebrew/opt/rustup/bin:$PATH cargo test -p cookbench-desktop --test local_notification_commands --test stove_events`

Expected: FAIL on missing commands and transition wiring.

**Step 3: Wire production behavior**

Register the plugin and commands. Load preferences from the existing config service, request banner permission only during explicit enable/test, and call local delivery after the reducer accepts an effective transition.

**Step 4: Run focused tests**

Run the command above again.

Expected: PASS with no alert from stale events.

**Step 5: Commit using the Lore protocol**

Commit command and lifecycle integration.

### Task 4: Add Local Alert Settings

**Files:**
- Modify: `src/settings/notifications/service.ts`
- Modify: `src/settings/notifications/service.test.ts`
- Modify: `src/settings/notifications/NotificationSettingsPanel.tsx`
- Modify: `src/settings/notifications/NotificationSettings.test.tsx`
- Modify: `src/settings/notifications/notification-settings.css`

**Step 1: Write failing component and service tests**

Assert sound is the only enabled default, every channel can be toggled and tested, the event selection is shared, and backend errors show transient nontechnical feedback.

**Step 2: Run focused tests and verify RED**

Run: `pnpm test --run src/settings/notifications`

Expected: FAIL because local alert controls and transports are missing.

**Step 3: Implement the Settings section**

Add compact toggles and icon-backed test buttons before outbound destinations. Preserve the existing General and Archive tabs and avoid nested cards.

**Step 4: Verify frontend settings**

Run:

```bash
pnpm test --run src/settings/notifications
pnpm lint
pnpm build
```

Expected: PASS.

**Step 5: Commit using the Lore protocol**

Commit the Settings UI and its tests.

### Task 5: Flash The Matching Stove

**Files:**
- Create: `src/services/localAlerts.ts`
- Create: `src/services/localAlerts.test.ts`
- Modify: `src/App.tsx`
- Modify: `src/components/GlobalBar.tsx`
- Modify: `src/components/DetachedStoveBar.tsx`
- Modify: `src/components/StoveBurner.tsx`
- Modify: `src/styles/global-bar.css`
- Modify: relevant component tests
- Modify: `tests/e2e/global-bar.spec.ts`

**Step 1: Write failing interaction tests**

Assert only the addressed Stove flashes, detached Stoves receive the same event, the effect expires, and reduced motion disables animation.

**Step 2: Run tests and verify RED**

Run: `pnpm test --run src/services/localAlerts.test.ts src/components`

Expected: FAIL on missing listener and flash state.

**Step 3: Implement the event hook and visual state**

Listen to the Tauri event once per window, expose the active Stove ID, add a stable data state to the matching burner, and implement a 1.2-second token-based emphasis without resizing the Bar.

**Step 4: Verify components and browser visuals**

Run:

```bash
pnpm test --run
pnpm test:e2e -- tests/e2e/global-bar.spec.ts
pnpm lint
pnpm build
```

Expected: PASS with no scrollbar, layout shift, or reduced-motion animation.

**Step 5: Commit using the Lore protocol**

Commit the flash integration and visual evidence.

### Task 6: Release Verification And Installation

**Files:**
- Modify: `docs/verification/platform-matrix.md` if platform evidence changes
- Modify: `docs/verification/release-checklist.md` if notification checks are absent

**Step 1: Run full verification**

Run:

```bash
PATH=/opt/homebrew/opt/rustup/bin:$PATH cargo fmt --all -- --check
PATH=/opt/homebrew/opt/rustup/bin:$PATH cargo clippy --workspace --all-targets -- -D warnings
PATH=/opt/homebrew/opt/rustup/bin:$PATH cargo test --workspace
pnpm test --run
pnpm lint
pnpm build
pnpm test:e2e
pnpm tauri build --debug --no-bundle
git diff --check
```

Expected: all automated checks PASS.

**Step 2: Perform native smoke tests**

On macOS, verify the packaged application plays the default sound, asks for banner permission only after an explicit action, flashes one Stove, and requests Dock attention. Verify settings survive restart.

On Windows and graphical Linux, verify sound, banner, and attention on native runners when available. Record unavailable hardware or permission checks honestly.

**Step 3: Install the verified macOS build**

Replace the existing `/Applications/Cookbench.app` only after the packaged build passes, relaunch it, and verify the running binary matches the new build.

**Step 4: Commit verification evidence using the Lore protocol**

Commit only durable documentation and evidence appropriate for the repository; do not commit user settings, notification history, credentials, or native Session data.
