# Cookbench Implementation Plan

> **For the implementing agent:** Use the `superpowers:executing-plans` workflow to implement this plan task-by-task.

**Goal:** Build a local-first desktop companion that discovers Codex, Claude Code, and Pi sessions, renders them as global and detachable stoves on macOS, Windows, and graphical Ubuntu, and supports optional SSH and outbound-only IM notifications.

**Architecture:** Use a Tauri 2 shell with a React/TypeScript UI, a shared Rust domain core, compiled-in first-party harness adapters, and a separate headless bridge binary. Native session files remain the source of truth; hooks improve immediacy; small atomic JSON files persist only Cookbench-owned layout and retained completion state.

**Tech Stack:** Rust stable, Tauri 2, React, TypeScript, Vite, Vitest, Playwright, serde/serde_json, tokio, notify, tracing, OS-native window APIs, system OpenSSH, and OS credential stores.

---

## Execution Rules

- Follow TDD for domain logic, parsers, state transitions, persistence, notifications, and remote protocols.
- Keep the main application usable after every task.
- Do not copy third-party source until its exact origin and license are recorded.
- Never use real user prompts or conversation content as committed fixtures.
- Do not add SQLite or a full conversation index.
- Do not expose inbound IM listeners or remote agent controls.
- Treat `docs/visual-prototype/` as the approved visual reference; preserve its
  bright light-default material, complete-ring rules, and lightweight asset budget.
- Commit after every task using the repository Lore commit protocol.
- Run `cargo fmt`, `cargo clippy`, Rust tests, frontend tests, and builds before each phase checkpoint.

## Proposed Repository Layout

```text
Cargo.toml
package.json
pnpm-lock.yaml
crates/
  cookbench-core/
  cookbench-adapters/
  cookbench-bridge/
  cookbench-hook/
src-tauri/
src/
tests/
  e2e/
  fixtures/
gnome-extension/
docs/
  plans/
  references/
scripts/
```

## Phase 0: Cross-Platform Risk Validation

### Task 1: Establish Reuse and License Guardrails

**Files:**
- Create: `THIRD_PARTY_NOTICES.md`
- Create: `docs/references/reuse-audit.md`
- Create: `docs/references/upstream-sources.md`

**Step 1: Write the reuse audit template**

Add columns for project, source URL, license, candidate files, intended use, copied/ported/idea-only status, modifications, and required notices.

**Step 2: Populate the initial decisions**

Record:

- CC Switch: MIT, build and packaging patterns only initially.
- CodeIsland: MIT, candidate hooks, normalizers, fixtures, and overlay behavior.
- DevIsland: MIT, provider boundary, IPC, terminal focus, and test patterns.
- AgentBar: MIT, atomic state protocol and hook fallback ideas.
- agent-status: Apache-2.0, provider/tailer/test ideas with notice obligations.
- CodexLens: MIT, Codex discovery and correlation ideas.
- Claude Status: BSD-3-Clause, macOS focus and process-correlation ideas.
- AgentHUD: no license file found; idea-only, no source copying.
- Vibe Kanban: Apache-2.0, product reference only.

**Step 3: Add the initial notice file**

State that no third-party source code has been incorporated yet. Leave a structured section for later additions.

**Step 4: Review the audit**

Run:

```bash
rg -n "AgentHUD|CodeIsland|CC Switch|license" THIRD_PARTY_NOTICES.md docs/references
```

Expected: every reviewed project is classified and AgentHUD is explicitly marked no-copy.

**Step 5: Commit**

```bash
git add THIRD_PARTY_NOTICES.md docs/references
git commit
```

### Task 2: Scaffold the Tauri/Rust Workspace

**Files:**
- Create: `Cargo.toml`
- Create: `package.json`
- Create: `pnpm-workspace.yaml`
- Create: `tsconfig.json`
- Create: `vite.config.ts`
- Create: `src-tauri/Cargo.toml`
- Create: `src-tauri/tauri.conf.json`
- Create: `src-tauri/src/main.rs`
- Create: `src-tauri/src/lib.rs`
- Create: `src/main.tsx`
- Create: `src/App.tsx`
- Create: `src/styles/tokens.css`
- Create: `src/styles/app.css`
- Create: `src/assets/cookbench-mark.svg`
- Create: `src/assets/cookbench-tray.svg`
- Create: `crates/cookbench-core/Cargo.toml`
- Create: `crates/cookbench-core/src/lib.rs`
- Create: `crates/cookbench-adapters/Cargo.toml`
- Create: `crates/cookbench-adapters/src/lib.rs`
- Create: `crates/cookbench-bridge/Cargo.toml`
- Create: `crates/cookbench-bridge/src/main.rs`
- Create: `crates/cookbench-hook/Cargo.toml`
- Create: `crates/cookbench-hook/src/main.rs`

**Step 1: Create a failing workspace smoke test**

In `crates/cookbench-core/src/lib.rs`, add:

```rust
#[cfg(test)]
mod tests {
    #[test]
    fn workspace_smoke_test() {
        assert_eq!(crate::PRODUCT_NAME, "Cookbench");
    }
}
```

**Step 2: Run the test and verify it fails**

Run:

```bash
cargo test -p cookbench-core workspace_smoke_test
```

Expected: compilation fails because `PRODUCT_NAME` is undefined.

**Step 3: Implement the minimal core and Tauri shell**

Define `pub const PRODUCT_NAME: &str = "Cookbench";`, configure the workspace
members, and scaffold a borderless Tauri window containing a static fake stove.
Copy the two approved original vector masters from `docs/visual-prototype/assets/`
into `src/assets/`; do not introduce raster UI art, a web-font dependency, or an
animation package.

The initial frontend test in `src/App.test.tsx` should assert that `Cookbench` and a `data-testid="stove"` element render.

**Step 4: Run all scaffold checks**

```bash
cargo test --workspace
pnpm test --run
pnpm build
cargo build --workspace
```

Expected: all commands pass and no product integration exists yet.

**Step 5: Commit**

```bash
git add Cargo.toml package.json pnpm-workspace.yaml tsconfig.json vite.config.ts src src-tauri crates
git commit
```

### Task 3: Prove the Overlay Contract on All Three Platforms

**Files:**
- Create: `src-tauri/src/platform/mod.rs`
- Create: `src-tauri/src/platform/overlay.rs`
- Create: `src-tauri/src/platform/macos.rs`
- Create: `src-tauri/src/platform/windows.rs`
- Create: `src-tauri/src/platform/linux.rs`
- Create: `src-tauri/src/platform/capabilities.rs`
- Create: `src-tauri/tests/platform_capabilities.rs`
- Create: `docs/verification/platform-overlay.md`

**Step 1: Write a failing capability test**

```rust
#[test]
fn wayland_without_extension_reports_best_effort_overlay() {
    let caps = capabilities_for(DesktopEnvironment::GnomeWayland, false);
    assert_eq!(caps.overlay, OverlaySupport::BestEffort);
    assert!(caps.graphical_ui);
}
```

Add equivalent tests for macOS, Windows, Ubuntu X11, and GNOME Wayland with the optional extension.

**Step 2: Run the test and verify it fails**

```bash
cargo test -p cookbench-desktop --test platform_capabilities
```

Expected: missing platform capability model.

**Step 3: Implement the platform-neutral overlay API**

Define:

```rust
pub trait OverlayController {
    fn show_global_bar(&self) -> Result<(), OverlayError>;
    fn show_detached(&self, stove_id: &str) -> Result<(), OverlayError>;
    fn set_always_on_top(&self, enabled: bool) -> Result<(), OverlayError>;
    fn move_to_display(&self, display_id: &str, x: f64, y: f64) -> Result<(), OverlayError>;
}
```

Implement a real platform spike:

- macOS: floating panel behavior without Accessibility permission.
- Windows: topmost borderless window without elevation.
- Ubuntu X11: keep-above request.
- Ubuntu Wayland: graphical fallback, with capability reporting instead of false success.

**Step 4: Perform manual platform verification**

Follow `docs/verification/platform-overlay.md` on real or VM environments. Record screenshots, OS versions, display scaling, full-screen behavior, and multi-monitor behavior.

Expected: full overlay on macOS/Windows/X11; usable graphical fallback on GNOME Wayland.

**Step 5: Commit**

```bash
git add src-tauri docs/verification/platform-overlay.md
git commit
```

## Phase 1: Local Core MVP

### Task 4: Implement the Domain Model and Stove State Machine

**Files:**
- Create: `crates/cookbench-core/src/domain/mod.rs`
- Create: `crates/cookbench-core/src/domain/event.rs`
- Create: `crates/cookbench-core/src/domain/identity.rs`
- Create: `crates/cookbench-core/src/domain/progress.rs`
- Create: `crates/cookbench-core/src/domain/stove.rs`
- Create: `crates/cookbench-core/src/state_machine.rs`
- Create: `crates/cookbench-core/tests/state_machine.rs`

**Step 1: Write failing transition tests**

Cover:

```rust
Cooking -> NeedsHuman -> Cooking -> Cooked
Cooked -> UserPromptSubmitted -> Cooking
Cooking -> ConnectionLost -> Disconnected
Disconnected -> ConnectionRestored -> previous state
Cooking -> recoverable ToolCompleted(error) -> Cooking
Cooking -> SessionFailed -> Failed
Cooked -> ClearRequested -> Removed
```

Also prove that inactivity alone cannot produce `Cooked`.

**Step 2: Run the tests and verify they fail**

```bash
cargo test -p cookbench-core --test state_machine
```

Expected: missing domain types and reducer.

**Step 3: Implement immutable event reduction**

Use a reducer shaped like:

```rust
pub fn reduce(previous: &Stove, event: &StoveEvent) -> Result<Stove, TransitionError>;
```

Include event source, confidence, sequence, timestamp, and supersession rules. Keep presentation labels out of the core.

**Step 4: Run focused and workspace tests**

```bash
cargo test -p cookbench-core --test state_machine
cargo test --workspace
```

Expected: all state transitions pass.

**Step 5: Commit**

```bash
git add crates/cookbench-core
git commit
```

### Task 5: Implement Atomic Cookbench-Owned Persistence

**Files:**
- Create: `crates/cookbench-core/src/persistence/mod.rs`
- Create: `crates/cookbench-core/src/persistence/config.rs`
- Create: `crates/cookbench-core/src/persistence/state.rs`
- Create: `crates/cookbench-core/src/persistence/atomic_file.rs`
- Create: `crates/cookbench-core/tests/persistence.rs`

**Step 1: Write failing persistence tests**

Test:

- Atomic replace never exposes partial JSON.
- Cooked stove summaries survive restart.
- No prompt body or full transcript field exists in the persisted schema.
- Clear cursors hide only events at or before the clear point.
- A newer prompt relights a cleared native session.
- Unknown future fields are tolerated.

**Step 2: Run tests and verify failure**

```bash
cargo test -p cookbench-core --test persistence
```

**Step 3: Implement minimal JSON persistence**

Use same-directory temporary files, flush, and atomic rename. Keep schemas versioned:

```rust
pub struct PersistedState {
    pub version: u32,
    pub retained: Vec<RetainedStove>,
    pub clear_cursors: Vec<ClearCursor>,
}
```

Do not introduce SQLite.

**Step 4: Run tests**

```bash
cargo test -p cookbench-core --test persistence
```

Expected: pass on macOS, Windows, and Linux CI.

**Step 5: Commit**

```bash
git add crates/cookbench-core/src/persistence crates/cookbench-core/tests/persistence.rs
git commit
```

### Task 6: Define the Harness Adapter Contract and Fixture Harness

**Files:**
- Create: `crates/cookbench-adapters/src/adapter.rs`
- Create: `crates/cookbench-adapters/src/capabilities.rs`
- Create: `crates/cookbench-adapters/src/registry.rs`
- Create: `crates/cookbench-adapters/src/fixture.rs`
- Create: `crates/cookbench-adapters/tests/contract.rs`
- Create: `tests/fixtures/README.md`

**Step 1: Write a failing adapter contract test**

Use a fixture adapter and assert that discovery, watch events, progress, locator, and resume capability are independently reported.

**Step 2: Run the test and verify it fails**

```bash
cargo test -p cookbench-adapters --test contract
```

**Step 3: Implement the contract**

```rust
#[async_trait]
pub trait HarnessAdapter: Send + Sync {
    fn id(&self) -> HarnessId;
    fn capabilities(&self) -> AdapterCapabilities;
    async fn discover(&self, source: &HostSource) -> Result<Vec<NativeSession>, AdapterError>;
    async fn watch(&self, sink: EventSink) -> Result<WatchHandle, AdapterError>;
    fn locate(&self, session: &NativeSession) -> Option<SessionLocator>;
    fn resume(&self, session: &NativeSession) -> Vec<ResumeAction>;
}
```

Do not force every adapter to claim unsupported features.

**Step 4: Run the contract tests**

```bash
cargo test -p cookbench-adapters --test contract
```

Expected: the fixture adapter passes the common contract.

**Step 5: Commit**

```bash
git add crates/cookbench-adapters tests/fixtures/README.md
git commit
```

### Task 7: Build Safe Incremental JSONL Tailing and Directory Watching

**Files:**
- Create: `crates/cookbench-adapters/src/io/mod.rs`
- Create: `crates/cookbench-adapters/src/io/jsonl_tailer.rs`
- Create: `crates/cookbench-adapters/src/io/directory_watch.rs`
- Create: `crates/cookbench-adapters/src/io/limits.rs`
- Create: `crates/cookbench-adapters/tests/jsonl_tailer.rs`

**Step 1: Write failing tailer tests**

Test appended records, partial final lines, truncation, rotation, oversized lines, invalid UTF-8, symlinks, and 1,000 historical files without loading their bodies.

**Step 2: Run tests and verify failure**

```bash
cargo test -p cookbench-adapters --test jsonl_tailer
```

**Step 3: Implement bounded incremental reading**

Track file identity, byte cursor, and the partial line buffer. Emit only complete bounded records. Never recurse through symlinks outside the configured root.

**Step 4: Run performance and correctness tests**

```bash
cargo test -p cookbench-adapters --test jsonl_tailer
```

Expected: no full-history loading and all malformed-input tests pass.

**Step 5: Commit**

```bash
git add crates/cookbench-adapters/src/io crates/cookbench-adapters/tests/jsonl_tailer.rs
git commit
```

### Task 8: Implement the Codex Adapter

**Files:**
- Create: `crates/cookbench-adapters/src/codex/mod.rs`
- Create: `crates/cookbench-adapters/src/codex/discovery.rs`
- Create: `crates/cookbench-adapters/src/codex/parser.rs`
- Create: `crates/cookbench-adapters/src/codex/progress.rs`
- Create: `crates/cookbench-adapters/src/codex/hook.rs`
- Create: `crates/cookbench-adapters/tests/codex.rs`
- Create: `tests/fixtures/codex/`

**Step 1: Create sanitized fixture generators**

Write a development-only sanitizer that keeps event types, IDs, timestamps, path shapes, and plan status while replacing prompt, output, command, and code content. Do not commit real local sessions.

**Step 2: Write failing Codex adapter tests**

Cover session discovery, `CODEX_HOME`, user prompt, tool activity, `update_plan`, successful turn completion, errors, and process/session correlation.

**Step 3: Run tests and verify failure**

```bash
cargo test -p cookbench-adapters --test codex
```

**Step 4: Implement read-only parsing first**

Parse only documented or fixture-backed record variants. Unknown records must not fail the session.

**Step 5: Add optional notify-hook integration**

Detect an existing Codex notify command. Do not overwrite it. Implement either safe explicit chaining or a read-only fallback. Add tests for both cases.

**Step 6: Run tests**

```bash
cargo test -p cookbench-adapters --test codex
cargo test --workspace
```

**Step 7: Commit**

```bash
git add crates/cookbench-adapters/src/codex crates/cookbench-adapters/tests/codex.rs tests/fixtures/codex
git commit
```

### Task 9: Implement the Claude Code Adapter

**Files:**
- Create: `crates/cookbench-adapters/src/claude/mod.rs`
- Create: `crates/cookbench-adapters/src/claude/discovery.rs`
- Create: `crates/cookbench-adapters/src/claude/parser.rs`
- Create: `crates/cookbench-adapters/src/claude/tasks.rs`
- Create: `crates/cookbench-adapters/src/claude/hooks.rs`
- Create: `crates/cookbench-adapters/tests/claude.rs`
- Create: `tests/fixtures/claude/`

**Step 1: Write failing discovery and parsing tests**

Cover default and `CLAUDE_CONFIG_DIR` roots, project path encoding, session titles, tasks/todos, tool lifecycle, permission, question, stop, error, and subagent records.

**Step 2: Run tests and verify failure**

```bash
cargo test -p cookbench-adapters --test claude
```

**Step 3: Implement native file support**

Use transcript and task records as read-only sources. Never write to Claude's native session files.

**Step 4: Implement merge-safe hooks**

Back up and structurally merge Cookbench hook entries. Preserve existing hooks and provide deterministic uninstall tests.

**Step 5: Run tests**

```bash
cargo test -p cookbench-adapters --test claude
```

Expected: native-only and hook-enhanced modes both pass.

**Step 6: Commit**

```bash
git add crates/cookbench-adapters/src/claude crates/cookbench-adapters/tests/claude.rs tests/fixtures/claude
git commit
```

### Task 10: Implement the Pi Adapter and Extension

**Files:**
- Create: `crates/cookbench-adapters/src/pi/mod.rs`
- Create: `crates/cookbench-adapters/src/pi/discovery.rs`
- Create: `crates/cookbench-adapters/src/pi/parser.rs`
- Create: `crates/cookbench-adapters/src/pi/extension.rs`
- Create: `crates/cookbench-adapters/tests/pi.rs`
- Create: `integrations/pi/cookbench.ts`
- Create: `tests/fixtures/pi/`

**Step 1: Write failing Pi tests**

Cover default and overridden session roots, versioned JSONL trees, session name, prompt, tool calls, extension lifecycle, todo custom entries, resume/fork identity, and normal completion.

**Step 2: Run tests and verify failure**

```bash
cargo test -p cookbench-adapters --test pi
```

**Step 3: Implement read-only Pi parsing**

Support versioned entry trees without mutating or migrating Pi sessions.

**Step 4: Implement the optional Pi extension**

The extension emits bounded lifecycle envelopes only. It must not register control tools or alter model context.

**Step 5: Run Rust and TypeScript integration tests**

```bash
cargo test -p cookbench-adapters --test pi
pnpm test --run integrations/pi
```

**Step 6: Commit**

```bash
git add crates/cookbench-adapters/src/pi crates/cookbench-adapters/tests/pi.rs integrations/pi tests/fixtures/pi
git commit
```

### Task 11: Implement the Cross-Platform Hook Helper

**Files:**
- Modify: `crates/cookbench-hook/src/main.rs`
- Create: `crates/cookbench-hook/src/envelope.rs`
- Create: `crates/cookbench-hook/src/spool.rs`
- Create: `crates/cookbench-hook/tests/hook.rs`
- Create: `docs/integrations/hooks.md`

**Step 1: Write failing CLI tests**

Pipe sanitized hook JSON into the binary and assert that it writes an atomic, bounded event envelope. Test missing Cookbench, full spool, malformed input, and unsupported event types.

**Step 2: Run tests and verify failure**

```bash
cargo test -p cookbench-hook
```

**Step 3: Implement a non-blocking helper**

The helper must return quickly, never wait for UI, and never break the host harness. Use a bounded runtime spool and explicit exit codes for diagnostics without exposing prompts.

**Step 4: Run tests and measure execution**

```bash
cargo test -p cookbench-hook
cargo run -p cookbench-hook -- --self-test
```

Expected: self-test passes and reports bounded execution time.

**Step 5: Commit**

```bash
git add crates/cookbench-hook docs/integrations/hooks.md
git commit
```

### Task 12: Connect the Rust State Store to Tauri Events

**Files:**
- Create: `src-tauri/src/app_state.rs`
- Create: `src-tauri/src/commands/stoves.rs`
- Create: `src-tauri/src/events.rs`
- Create: `src/types/stove.ts`
- Create: `src/services/stoves.ts`
- Create: `src/hooks/useStoves.ts`
- Create: `src-tauri/tests/stove_events.rs`
- Create: `src/services/stoves.test.ts`

**Step 1: Write failing serialization tests**

Assert that the Rust and TypeScript wire models include harness, host, state, progress provenance, locator capability, and retained completion status without raw transcript fields.

**Step 2: Run tests and verify failure**

```bash
cargo test -p cookbench-desktop --test stove_events
pnpm test --run src/services/stoves.test.ts
```

**Step 3: Implement snapshot and incremental event commands**

Expose a snapshot command for startup and ordered incremental events afterward. Handle event gaps by requesting a new snapshot.

**Step 4: Run tests**

```bash
cargo test -p cookbench-desktop --test stove_events
pnpm test --run src/services/stoves.test.ts
```

**Step 5: Commit**

```bash
git add src-tauri/src/app_state.rs src-tauri/src/commands src-tauri/src/events.rs src/types src/services src/hooks
git commit
```

### Task 13: Build the Global Dynamic Stove Bar

**Files:**
- Create: `src/components/GlobalBar.tsx`
- Create: `src/components/StoveBurner.tsx`
- Create: `src/components/StoveTooltip.tsx`
- Create: `src/components/HarnessMark.tsx`
- Create: `src/components/ProgressRing.tsx`
- Create: `src/components/HostBadge.tsx`
- Create: `src/components/global-bar.css`
- Create: `src/components/GlobalBar.test.tsx`
- Create: `src/stories/GlobalBar.fixture.ts`

**Step 1: Write failing component tests**

Test that:

- Every stove renders simultaneously.
- Burner count follows session count.
- Every burner visibly identifies Codex, Claude Code, or Pi.
- Determinate progress is used only with structured provenance.
- Attention, Cooked, Failed, and Disconnected each use a complete ring and never
  use different arc lengths to encode their state.
- The complete static rings use state colors, center labels, and accessible names.
- Only Cooking with structured provenance may use an incomplete determinate arc.
- Indeterminate Cooking does not display an invented numeric value.
- High counts wrap or scale without hiding stoves.

**Step 2: Run tests and verify failure**

```bash
pnpm test --run src/components/GlobalBar.test.tsx
```

**Step 3: Implement the stable layout**

Use fixed burner aspect ratios, bounded responsive tracks, zero negative letter
spacing, and no viewport-scaled font sizes. Keep hover content outside burner
layout calculations so it cannot shift the bar. Derive tokens and ring behavior
from `docs/visual-prototype/`; use system fonts, CSS motion, and the approved SVGs.
The default Bar is a bright translucent functional layer, not a dark control
surface. Implement platform-aware native blur/backdrop capability with a solid
high-opacity fallback; do not stack glass surfaces or imitate macOS window chrome
on Windows and Ubuntu.

**Step 4: Run tests and screenshot fixtures**

```bash
pnpm test --run src/components/GlobalBar.test.tsx
pnpm test:e2e --grep "global bar fixtures"
```

Expected: 1, 6, 10, 20, and 30-stove screenshots have no overlap or hidden burners.

**Step 5: Commit**

```bash
git add src/components src/stories tests/e2e
git commit
```

### Task 14: Implement Detachable Stove Bars and Layout Persistence

**Files:**
- Create: `src/components/DetachedStoveBar.tsx`
- Create: `src/components/DetachedStoveBar.test.tsx`
- Create: `src-tauri/src/commands/windows.rs`
- Create: `src-tauri/src/window_registry.rs`
- Create: `crates/cookbench-core/src/persistence/layout.rs`
- Create: `src-tauri/tests/window_registry.rs`

**Step 1: Write failing window-registry tests**

Test one detached window per stove, coexistence with the global bar, display-relative positioning, window restoration, and cleanup after manual clear.

**Step 2: Run tests and verify failure**

```bash
cargo test -p cookbench-desktop --test window_registry
pnpm test --run src/components/DetachedStoveBar.test.tsx
```

**Step 3: Implement detached window commands**

Use the same Stove wire model and visual state tokens as the global burner. Persist monitor identity and relative coordinates, not fragile absolute global pixels alone.

**Step 4: Verify multi-monitor behavior manually**

Test detach, drag, sleep/wake, monitor removal, restart, and clear on all platforms.

**Step 5: Commit**

```bash
git add src/components/DetachedStoveBar* src-tauri/src/commands/windows.rs src-tauri/src/window_registry.rs crates/cookbench-core/src/persistence/layout.rs src-tauri/tests/window_registry.rs
git commit
```

### Task 15: Implement Session Locators and Jump Fallbacks

**Files:**
- Create: `crates/cookbench-core/src/locator/mod.rs`
- Create: `crates/cookbench-core/src/locator/model.rs`
- Create: `src-tauri/src/locator/mod.rs`
- Create: `src-tauri/src/locator/macos.rs`
- Create: `src-tauri/src/locator/windows.rs`
- Create: `src-tauri/src/locator/linux.rs`
- Create: `src-tauri/src/locator/tmux.rs`
- Create: `src-tauri/src/locator/vscode.rs`
- Create: `src-tauri/tests/locator_fallback.rs`

**Step 1: Write failing fallback-order tests**

Assert:

```text
exact pane -> application window -> project directory -> resume instructions
```

Also test permission denial and elevated target applications.

**Step 2: Run tests and verify failure**

```bash
cargo test -p cookbench-desktop --test locator_fallback
```

**Step 3: Implement bounded first-party locators**

Start with tmux, VS Code, macOS Terminal/iTerm, Windows Terminal best effort, and common Ubuntu terminal fallback. Do not pretend unsupported terminals have exact-tab support.

**Step 4: Run tests and manual host checks**

Record the host capability matrix in `docs/verification/session-focus.md`.

**Step 5: Commit**

```bash
git add crates/cookbench-core/src/locator src-tauri/src/locator src-tauri/tests/locator_fallback.rs docs/verification/session-focus.md
git commit
```

### Task 16: Add Local Notifications and Cooked Feedback

**Files:**
- Create: `crates/cookbench-core/src/notifications/event.rs`
- Create: `src-tauri/src/notifications/local.rs`
- Create: `src/animation/stoveMotion.ts`
- Create: `src/animation/stoveMotion.test.ts`
- Create: `src/components/StoveBurner.motion.test.tsx`

**Step 1: Write failing feedback tests**

Test one completion effect per Cooked transition, no effect on stale replay,
optional sound, reduced-motion behavior, and local notification permission denial.
Assert that the completion animation settles on a static complete green ring and
does not keep rotating.

**Step 2: Run tests and verify failure**

```bash
pnpm test --run src/animation src/components/StoveBurner.motion.test.tsx
```

**Step 3: Implement restrained feedback**

Use a short CSS completion state with no layout shift. Keep sound off or
conservative by default and user-configurable. Do not add GIF, video, Lottie,
sprite, or decorative raster assets.

**Step 4: Verify manually**

Test repeated corrections, relighting, completion, and clearing on all platforms.

**Step 5: Commit**

```bash
git add crates/cookbench-core/src/notifications src-tauri/src/notifications src/animation src/components/StoveBurner.motion.test.tsx
git commit
```

## Phase 2: Remote Sources and Outbound Notifications

### Task 17: Implement the Notification Rule Engine

**Files:**
- Create: `crates/cookbench-core/src/notifications/rules.rs`
- Create: `crates/cookbench-core/src/notifications/template.rs`
- Create: `crates/cookbench-core/src/notifications/dedupe.rs`
- Create: `crates/cookbench-core/src/notifications/queue.rs`
- Create: `crates/cookbench-core/tests/notification_rules.rs`

**Step 1: Write failing rule-engine tests**

Cover global, project, host, harness, destination, and stove overrides; milestone filtering; safe placeholders; private-field exclusion; deduplication; coalescing; bounded queue; retry expiry; and critical-event priority.

**Step 2: Run tests and verify failure**

```bash
cargo test -p cookbench-core --test notification_rules
```

**Step 3: Implement pure deterministic evaluation**

Keep rule evaluation free of network I/O. Reject unknown placeholders and bound rendered message length.

**Step 4: Run tests**

```bash
cargo test -p cookbench-core --test notification_rules
```

**Step 5: Commit**

```bash
git add crates/cookbench-core/src/notifications crates/cookbench-core/tests/notification_rules.rs
git commit
```

### Task 18: Implement Outbound-Only Notification Adapters

**Files:**
- Create: `src-tauri/src/notifications/sender.rs`
- Create: `src-tauri/src/notifications/telegram.rs`
- Create: `src-tauri/src/notifications/slack.rs`
- Create: `src-tauri/src/notifications/discord.rs`
- Create: `src-tauri/src/notifications/lark.rs`
- Create: `src-tauri/src/notifications/generic.rs`
- Create: `src-tauri/src/secrets.rs`
- Create: `src-tauri/tests/outbound_notifications.rs`
- Create: `src/settings/notifications/NotificationSettings.tsx`
- Create: `src/settings/notifications/NotificationSettings.test.tsx`

**Step 1: Write failing mock-server tests**

Verify exact outbound payloads, timeouts, retry classification, rate limiting, channel isolation, credential redaction, and that no HTTP listener or polling loop is created.

**Step 2: Run tests and verify failure**

```bash
cargo test -p cookbench-desktop --test outbound_notifications
```

**Step 3: Implement Telegram, Slack, Discord, Lark, and generic webhook senders**

Keep platform-specific payload mapping behind a common outbound trait. Store secrets in the OS credential store; store only references in config.

**Step 4: Implement settings and test-send actions**

Test-send must use a synthetic message and never expose a secret in the UI.

**Step 5: Run all notification tests**

```bash
cargo test -p cookbench-desktop --test outbound_notifications
pnpm test --run src/settings/notifications
```

**Step 6: Commit**

```bash
git add src-tauri/src/notifications src-tauri/src/secrets.rs src-tauri/tests/outbound_notifications.rs src/settings/notifications
git commit
```

### Task 19: Implement Zero-Install SSH Sources

**Files:**
- Create: `crates/cookbench-core/src/remote/mod.rs`
- Create: `crates/cookbench-core/src/remote/host.rs`
- Create: `src-tauri/src/remote/ssh.rs`
- Create: `src-tauri/src/remote/zero_install.rs`
- Create: `src-tauri/src/remote/reconnect.rs`
- Create: `src-tauri/tests/ssh_source.rs`
- Create: `tests/ssh/docker-compose.yml`
- Create: `tests/ssh/fixtures/`

**Step 1: Write failing SSH-source tests**

Cover existing SSH config use, host-key verification, read-only discovery, remote custom session roots, adaptive polling, disconnection, reconnection, path collision, and absence of remote writes.

**Step 2: Run tests and verify failure**

```bash
cargo test -p cookbench-desktop --test ssh_source
```

**Step 3: Implement system OpenSSH transport**

Use the user's existing OpenSSH configuration and `known_hosts`. Do not implement password storage. Invoke only fixed read-only remote probes with strict argument boundaries.

**Step 4: Run isolated SSH tests**

```bash
docker compose -f tests/ssh/docker-compose.yml up -d
cargo test -p cookbench-desktop --test ssh_source
docker compose -f tests/ssh/docker-compose.yml down
```

Expected: all remote tests pass without opening a Cookbench port.

**Step 5: Commit**

```bash
git add crates/cookbench-core/src/remote src-tauri/src/remote src-tauri/tests/ssh_source.rs tests/ssh
git commit
```

### Task 20: Implement the Temporary Remote Bridge

**Files:**
- Modify: `crates/cookbench-bridge/src/main.rs`
- Create: `crates/cookbench-bridge/src/protocol.rs`
- Create: `crates/cookbench-bridge/src/server.rs`
- Create: `crates/cookbench-bridge/tests/protocol.rs`
- Create: `src-tauri/src/remote/bridge.rs`
- Create: `src-tauri/tests/bridge_source.rs`

**Step 1: Write failing protocol tests**

Cover JSONL LF framing, hello/version negotiation, capability exchange, normalized events, heartbeat, bounded record length, graceful shutdown, protocol mismatch, and corrupted input.

**Step 2: Run tests and verify failure**

```bash
cargo test -p cookbench-bridge
cargo test -p cookbench-desktop --test bridge_source
```

**Step 3: Implement the read-only bridge**

The bridge may discover and parse sessions but must expose no command that writes files, sends prompts, approves tools, or starts agents.

**Step 4: Implement checksum-verified temporary deployment**

Upload only after explicit user selection, verify the binary hash, start through SSH stdio, and stop it with the connection.

**Step 5: Run protocol and lifecycle tests**

```bash
cargo test -p cookbench-bridge
cargo test -p cookbench-desktop --test bridge_source
```

**Step 6: Commit**

```bash
git add crates/cookbench-bridge src-tauri/src/remote/bridge.rs src-tauri/tests/bridge_source.rs
git commit
```

### Task 21: Add the Optional GNOME Shell Presentation Extension

**Files:**
- Create: `gnome-extension/metadata.json`
- Create: `gnome-extension/extension.js`
- Create: `gnome-extension/stylesheet.css`
- Create: `gnome-extension/prefs.js`
- Create: `gnome-extension/tests/`
- Create: `src-tauri/src/platform/gnome_bridge.rs`
- Create: `docs/integrations/gnome-extension.md`

**Step 1: Write protocol and rendering fixture tests**

The extension receives sanitized presentation-only stove summaries and renders all stoves. It must not read harness sessions, credentials, or notification settings.

**Step 2: Run extension tests and verify failure**

Use the supported GNOME extension test harness for the target Ubuntu releases.

**Step 3: Implement a versioned presentation bridge**

Keep the main application authoritative. The extension must degrade cleanly when Cookbench is absent.

**Step 4: Verify on Ubuntu 22.04 and 24.04 GNOME Wayland**

Test install, enable, disable, GNOME restart/session restart, multiple monitors, and extension removal.

**Step 5: Commit**

```bash
git add gnome-extension src-tauri/src/platform/gnome_bridge.rs docs/integrations/gnome-extension.md
git commit
```

## Phase 3: Hardening, Packaging, and Public Beta

### Task 22: Add Security, Diagnostics, and Resource Budgets

**Files:**
- Create: `crates/cookbench-core/src/diagnostics.rs`
- Create: `src-tauri/src/diagnostics.rs`
- Create: `src-tauri/tests/redaction.rs`
- Create: `tests/performance/idle.rs`
- Create: `tests/performance/session_scale.rs`
- Create: `docs/security.md`
- Create: `docs/privacy.md`

**Step 1: Write failing redaction and isolation tests**

Assert that prompts, code, tokens, webhook URLs, credentials, and SSH secrets never enter diagnostics. Fuzz or property-test malformed adapter records and notification payloads.

**Step 2: Run tests and verify failure**

```bash
cargo test -p cookbench-desktop --test redaction
```

**Step 3: Implement structured redacted diagnostics**

Include adapter health, capability status, parser error counts, source paths with user-home redaction, and platform fallback reasons.

**Step 4: Add performance checks**

Measure idle CPU, memory, hook-to-UI latency, 1,000 historical sessions, and 30 active stoves. Record baselines rather than relying on anecdotal observation.

**Step 5: Run the hardening suite**

```bash
cargo test --workspace
pnpm test --run
```

Expected: redaction and isolation tests pass; performance results meet or explicitly document deviations from targets.

**Step 6: Commit**

```bash
git add crates/cookbench-core/src/diagnostics.rs src-tauri/src/diagnostics.rs src-tauri/tests/redaction.rs tests/performance docs/security.md docs/privacy.md
git commit
```

### Task 23: Build Cross-Platform E2E and Visual Verification

**Files:**
- Create: `tests/e2e/global-bar.spec.ts`
- Create: `tests/e2e/detached-bars.spec.ts`
- Create: `tests/e2e/stove-lifecycle.spec.ts`
- Create: `tests/e2e/notifications.spec.ts`
- Create: `tests/e2e/fixtures.ts`
- Create: `docs/verification/release-checklist.md`

**Step 1: Write failing end-to-end flows**

Simulate:

- Three harness sessions appearing from original tools.
- All harness sources visible in the global bar.
- Needs Human and Cooked transitions.
- A Cooked stove surviving restart.
- Relighting after a new prompt.
- Detach, reposition, restore, and clear.
- SSH disconnect without false completion.
- Filtered outbound notification.
- Full static rings for Attention, Cooked, Failed, and Disconnected.
- A determinate arc only for Cooking with structured progress.
- An indeterminate Cooking fixture with no numeric percentage.

**Step 2: Run tests and verify failure**

```bash
pnpm test:e2e
```

**Step 3: Complete missing integration behavior**

Fix only behavior required by the approved acceptance criteria. Avoid adding dashboard, chat, or orchestration scope.

**Step 4: Run desktop screenshots at required viewports and scales**

Verify no text overflow, overlap, blank windows, layout shifts, or hidden stoves
at supported scales. Compare the rendered global bar, detached bars, mark, source
labels, bright material, vivid colors, and state rings with
`docs/visual-prototype/index.html`. Verify light and dark desktop backgrounds,
reduced transparency, increased contrast, reduced motion, and blur-unavailable
fallbacks.

**Step 5: Commit**

```bash
git add tests/e2e docs/verification/release-checklist.md
git commit
```

### Task 24: Add CI, Packaging, Signing Guidance, and Release Artifacts

**Files:**
- Create: `.github/workflows/ci.yml`
- Create: `.github/workflows/release.yml`
- Create: `scripts/verify.sh`
- Create: `scripts/package-smoke.sh`
- Create: `docs/releasing.md`
- Create: `docs/installing.md`
- Modify: `src-tauri/tauri.conf.json`

**Step 1: Add failing CI matrix checks**

The matrix must include macOS, Windows, and Ubuntu for:

```bash
cargo fmt --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
pnpm lint
pnpm test --run
pnpm build
cargo build --workspace
```

**Step 2: Add packaging smoke tests**

Validate DMG/app bundle, MSI, DEB, and AppImage contents. Confirm the bridge and
hook helper match platform architecture and carry expected version metadata.
Fail the package audit if runtime photos, GIFs, videos, Lottie files, sprite
sheets, bundled web fonts, or unreviewed third-party brand artwork appear.

**Step 3: Document signing and permissions**

Document macOS signing/notarization, Windows signing, Linux package dependencies, hook locations, credential-store requirements, and GNOME extension installation.

**Step 4: Run the complete verification locally where possible**

```bash
./scripts/verify.sh
```

Expected: all available lint, test, and build checks pass. Record platform-only gaps honestly.

**Step 5: Commit**

```bash
git add .github scripts docs/releasing.md docs/installing.md src-tauri/tauri.conf.json
git commit
```

## Final Verification

Before declaring public beta readiness:

1. Compare the finished behavior with all 17 acceptance criteria in the approved design.
2. Review every third-party reuse entry and update `THIRD_PARTY_NOTICES.md`.
3. Inspect the full diff for accidental conversation fixtures, credentials, or generated artifacts.
4. Run the complete CI-equivalent verification on macOS, Windows, and Ubuntu.
5. Run real Codex, Claude Code, and Pi sessions from their original tools.
6. Verify Hooks uninstall without altering unrelated user configuration.
7. Verify zero-install SSH and bridge modes against an isolated remote host.
8. Verify Telegram, Slack, Discord, Lark, and generic webhook integrations with sandbox destinations.
9. Verify that no inbound IM endpoint, message polling, or agent-control action exists.
10. Record residual Wayland and exact-window-focus limitations in release notes.
