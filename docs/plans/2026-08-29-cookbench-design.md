# Cookbench Product and Technical Design

Date: 2026-08-29
Status: Approved design

## 1. Product Definition

Cookbench is a lightweight, cross-platform companion for existing coding harnesses. It does not replace Codex, Claude Code, Pi, terminals, IDEs, or chat surfaces. Users continue to start and operate agents in their original tools. Cookbench discovers those sessions, normalizes their state, presents them as "stoves," and returns users to the original tool when intervention is needed.

Supported desktop platforms:

- macOS
- Windows
- Graphical Ubuntu/Linux environments, including X11 and Wayland

Initial harness support:

- Codex
- Claude Code
- Pi

Planned adapter entry points include Gemini CLI, OpenCode, Cursor, DeepSeek harnesses, OpenClaw, Hermes Agent, Grok Build, and other tools implementing the Cookbench adapter protocol.

## 2. Product Goals

- Show every current or retained coding session at a glance.
- Preserve the original coding harness as the place where users chat and work.
- Distinguish cooking, human-attention, completion, failure, and connectivity states reliably.
- Show real progress only when the source exposes a structured plan or task list.
- Jump back to the original terminal, IDE, application, SSH session, or best available fallback.
- Support a global multi-stove bar and detachable per-session desktop bars.
- Work locally and across SSH-connected remote hosts.
- Send configurable, outbound-only status notifications to common IM tools.
- Remain local-first, low-resource, and non-invasive.

## 3. Non-Goals

The first product version will not:

- Embed chat, a terminal, or a code editor.
- Launch, orchestrate, or host coding agents.
- Manage model providers or API routing.
- Copy or index complete conversations in a Cookbench database.
- Fabricate percentage progress without a structured source.
- Provide team accounts or cloud synchronization.
- Receive IM messages or control agents through IM.
- Modify or delete native harness session history when a stove is cleared.

## 4. Domain Model

```text
Cookbench
  +-- Host: the local computer or an SSH host
  +-- Project: a Git root or working directory on a host
  +-- Stove: one harness session, treated as one task
        +-- multiple turns and corrections
        +-- plans and task items
        +-- tool activity
        +-- human interventions
        +-- final result
```

Identity rules:

```text
Stove ID  = Host identity + Harness + Native session ID
Project ID = Host identity + Canonical Git root or working directory
```

A new prompt in a completed native session relights the same stove. A new native session creates a new stove, even if it belongs to the same project.

## 5. Interaction Design

### 5.1 Global Cookbench Bar

The default surface is a floating global bar. It is a dashboard rather than a priority-only notification strip.

- The bar displays every active or retained stove simultaneously.
- The number of circular burners grows and shrinks with the number of stoves.
- There are no empty capacity slots.
- The bar never collapses to a single "most important" stove.
- At higher counts, the layout may reduce burner size or use multiple rows, but it must not hide running stoves.
- Each burner always displays the source harness in its center.
- The outer ring communicates reliable progress when available.
- Flame, motion, shape, and text communicate state without relying on color alone.
- A small badge distinguishes local and remote hosts.

Hovering a burner shows:

- Project and task title
- Local or remote host
- Source harness
- Current phase and activity
- Structured progress, when available
- Elapsed time
- Whether human intervention is required

Clicking a burner returns the user to the original work surface using the most precise available locator.

### 5.2 Detachable Stove Bars

Any stove can also render as an independent floating desktop bar.

- Independent bars can be dragged and placed like desktop notes.
- They can be placed on different monitors.
- The global bar and independent bars can coexist.
- Users can hide the global bar and retain only independent bars.
- Both surfaces share the same stove model, state vocabulary, progress, harness identity, and click behavior.

### 5.3 Completion and Clearing

Successful turn completion immediately produces a Cooked state.

- Cooked triggers a restrained completion animation, optional sound, and local notification.
- A cooked stove remains indefinitely until the user manually clears it.
- Clearing removes only Cookbench's retained presentation state.
- Native session files and conversations are never deleted.
- A new prompt in the same session relights the stove automatically.

## 6. Stove State Model

User-visible states:

```text
Starting
Planning
Cooking
Needs Human
Cooked
Failed
Disconnected
```

State precedence:

1. Explicit question or permission request -> Needs Human
2. Explicit final session failure -> Failed
3. Explicit successful turn completion -> Cooked
4. Active generation or tool use -> Cooking
5. Explicit planning activity -> Planning
6. Lost remote data source -> Disconnected

Inactivity alone never means Cooked or Failed. A single failed tool call does not fail a stove because the agent may recover.

Normalized events:

```text
SessionDiscovered
UserPromptSubmitted
PlanUpdated
ToolStarted
ToolCompleted
QuestionAsked
PermissionRequested
TurnCompleted
SessionFailed
ProcessExited
ConnectionLost
ConnectionRestored
```

Every event carries its source, timestamp, confidence, and sequence information. Explicit hooks outrank structured session records, which outrank process state and time-based inference. Old high-priority events expire when superseded by newer authoritative events.

## 7. Progress Rules

Cookbench displays determinate progress only from reliable structured sources, such as:

- Codex `update_plan`
- Claude Code tasks or todo state
- Pi todo extensions or structured plan records

If four of six plan items are complete, Cookbench can display `4/6` and a progress ring. If no structured plan exists, the UI displays current activity and an indeterminate cooking treatment without a percentage.

Plan changes are accepted. The current plan is authoritative and the total item count does not need to remain fixed.

## 8. System Architecture

```text
Tauri 2 desktop application
  +-- Rust core
  +-- React + TypeScript UI
  +-- Harness adapters
  +-- Platform overlay adapters
  +-- Remote SSH sources
  +-- Notification router
```

### 8.1 Rust Core

The shared Rust core is responsible for:

- Local and remote session discovery
- Incremental session-file watching
- Hook and Pi extension events
- Event normalization
- Stove state transitions
- Git root detection and project grouping
- Process and host application correlation
- SSH connections and bridge transport
- Bounded persistence of Cookbench-owned state
- Real-time UI events

### 8.2 Harness Adapter Contract

Each harness adapter implements:

```text
discover   Find existing native sessions
watch      Observe session files and lifecycle events
normalize  Produce Cookbench events
progress   Parse structured plans and tasks
locate     Correlate a session with its host surface
resume     Produce the best available jump or resume action
```

Initial adapters are compiled into the application. A future third-party adapter protocol will use external processes and JSONL framing instead of loading untrusted native libraries into Cookbench.

### 8.3 Data Sources and Precedence

Adapters combine:

```text
Native session files  Authoritative history and structured task data
Hooks/extensions      Immediate lifecycle and attention events
Process trees         Liveness and host correlation
Terminal/IDE metadata Jump location
Optional Skill/MCP    Semantic enhancement
```

Cookbench does not require the optional Skill or MCP to provide baseline monitoring.

## 9. Local Persistence

Cookbench does not use SQLite in the first version and does not copy native conversations.

```text
~/.cookbench/config.json
~/.cookbench/state.json
```

`config.json` contains:

- Bar mode and placement
- Independent stove positions
- Enabled harnesses
- References to SSH host configuration
- Notification, sound, and display preferences

`state.json` contains only Cookbench-owned state:

- Cooked stoves not yet cleared
- Clear cursors and timestamps
- Project aliases
- Last reliable summary state
- Minimum session locator information

Writes are atomic. A cleared-session cursor allows newer activity in the same native session to relight the stove.

## 10. Session Discovery and Recovery

Default session sources include:

```text
Codex       ~/.codex/sessions/**/*.jsonl
Claude Code ~/.claude/projects/**/*.jsonl
Pi          ~/.pi/agent/sessions/**/*.jsonl
```

Adapters also resolve supported custom configuration and session directories.

Title selection order:

1. Native session name
2. Native generated title
3. A bounded, local rendering of the first user request
4. Project directory name

Cookbench does not invoke an LLM merely to title a session.

JSONL files are tailed incrementally. On restart, active sessions are reconstructed from native data, retained cooked stoves are restored from Cookbench state, stale transient hook data is discarded, and remote sources reconnect.

## 11. Jump Back to the Original Tool

Cookbench records the best available locator:

```text
processId
parentProcessId
workingDirectory
hostApplication
terminal
tty
tmuxPane
IDE workspace
native session ID
```

Jump behavior degrades in this order:

1. Exact terminal tab, pane, or IDE session
2. Correct application window
3. Correct project directory
4. A visible resume command or native session ID

Cookbench never embeds the conversation. After the jump, the user continues chatting in the original tool.

## 12. Remote SSH Support

Remote hosts use two compatible modes.

### 12.1 Zero-Install Mode

Cookbench uses the user's existing SSH configuration to inspect native session files and process state. This mode requires no remote service but may have lower event fidelity and slower refresh.

### 12.2 Optional Bridge Mode

A small, single-file `cookbench-bridge` runs temporarily on the remote host.

- It watches and parses remote native sessions.
- It emits normalized events over SSH standard input/output.
- It does not open a listening network port.
- It does not persist complete conversations.
- It does not invoke or control agents.
- It exits with the local connection unless explicitly configured otherwise in a future version.

Remote hosts are read-only by default. Cookbench does not automatically use `sudo`, approve operations, or send remote commands.

When SSH disconnects, affected stoves enter Disconnected and retain their last known state. Disconnection never implies completion. Reconnection restores the same stove identities.

Clicking a remote stove first attempts to focus the original local SSH terminal or tmux pane. If it was discovered passively, Cookbench opens the preferred terminal and provides the best native resume or attach path.

## 13. Outbound-Only IM Notifications

The notification router consumes validated stove state transitions after the state engine. It never receives IM messages and never exposes agent controls through IM.

Initial notification adapters:

- Telegram Bot `sendMessage`
- Slack Incoming Webhook
- Discord Incoming Webhook
- Lark/Feishu custom bot webhook
- Generic HTTP webhook

Later adapters:

- Microsoft Teams Workflow webhook
- WhatsApp Business Cloud API

WhatsApp is an advanced setup because official proactive messaging requires Meta Business configuration, credentials, a sending number, and commonly approved message templates. Cookbench will not use browser automation or personal-account workarounds.

Configurable notification events:

```text
Session appeared
Cooking started
Phase changed
Needs Human
Progress milestone
Cooked
Failed
SSH disconnected
SSH restored
Stove cleared
```

Rules may be global or scoped by project, host, harness, destination, or stove. Templates use bounded placeholders such as:

```text
{project} {task} {agent} {state} {progress}
{activity} {host} {duration} {completed_at}
```

Absolute paths, full prompts, commands, and code are excluded by default. Repeated states are deduplicated, rapid transitions are coalesced, progress can be limited to milestones, retries are bounded, and notification failures never block stove processing.

Secrets are stored in macOS Keychain, Windows Credential Manager, or Ubuntu Secret Service/libsecret. Configuration files store only credential references.

## 14. Platform Overlay Design

Cookbench uses a shared overlay interface with platform-specific implementations.

| Capability | macOS | Windows | Ubuntu X11 | Ubuntu Wayland |
| --- | --- | --- | --- | --- |
| Graphical application | Full | Full | Full | Full |
| Global always-on-top bar | Full | Full | Near full | Best effort |
| Independent stove bars | Full | Full | Near full | Best effort |
| System notifications | Full | Full | Full | Full |
| Optional extension for full overlay | N/A | N/A | Not needed | GNOME Shell extension |
| Exact jump | Host-adapter dependent | Host-adapter dependent | Host-adapter dependent | Desktop-policy limited |

Implementation direction:

- macOS: AppKit `NSPanel` and appropriate collection behavior
- Windows: Win32 topmost window handling
- Ubuntu X11: EWMH keep-above behavior
- Ubuntu Wayland: best available application window plus optional GNOME Shell extension

The GNOME extension is a presentation bridge only. It does not parse sessions or own stove state. The main application remains fully graphical and functional without it.

## 15. Permissions and Safety

Permissions are progressive and optional where possible.

- Basic macOS floating UI requires no Accessibility permission.
- Notifications are requested only when enabled.
- Exact macOS application or terminal targeting may request Automation or Accessibility depending on the host.
- Windows does not require elevation for its own topmost window; elevated target applications may reduce jump precision.
- Ubuntu Wayland restrictions degrade presentation and focus, not monitoring.

Hook installation must parse and preserve native configuration, show the intended modification, write atomically, back up before writing, and support clean removal. Existing hooks are preserved. When a harness exposes only a single callback and safe chaining is impossible, Cookbench falls back to native session monitoring rather than overwriting user configuration.

## 16. Reliability and Resource Controls

- File-system events are preferred over full-directory polling.
- JSONL tailers read only appended data.
- Active sessions receive higher refresh priority than cooked stoves.
- Remote zero-install polling is adaptive; bridge mode is event-driven.
- Each adapter isolates malformed or unknown session records.
- Record size, nesting, line length, and field length are bounded.
- Unknown fields are ignored safely.
- A single broken adapter or session cannot crash Cookbench.
- Diagnostic logs redact prompts, code, credentials, and webhook secrets.
- Animation pauses when hidden or when reduced-motion settings are enabled.
- High stove counts reduce animation refresh rate rather than hiding stoves.

Target performance:

- Idle CPU below 1% on supported reference systems
- Resident memory target below 150 MB
- Local hook-to-UI update below 200 ms
- Bridge event update normally below 500 ms, excluding network latency
- Zero-install SSH state refresh normally within 5 seconds
- No full in-memory loading of 1,000 historical sessions
- Smooth operation with 30 active stoves

## 17. Testing Strategy

### 17.1 Adapter Contract Tests

Each adapter uses sanitized fixtures to validate discovery, incremental parsing, project/title detection, task progress, attention events, completion, abnormal exit, damaged records, and upstream format variants.

### 17.2 State Machine Tests

Required transitions include:

```text
Cooking -> Needs Human -> Cooking -> Cooked
Cooked -> New Prompt -> Cooking
Cooking -> SSH disconnected -> Restored prior state
Cooking -> Tool failure -> Cooking
Cooking -> Final session failure -> Failed
Cooked -> Manual clear -> Removed
```

### 17.3 Notification Tests

Tests cover filters, templates, deduplication, coalescing, bounded retries, credential redaction, channel isolation, and absence of inbound message listeners.

### 17.4 Remote Tests

Tests cover zero-install SSH inspection, bridge lifecycle, protocol mismatch, disconnect/reconnect, host-key changes, remote session restart, and local/remote path collisions.

### 17.5 Cross-Platform Verification

Release verification covers macOS Intel and Apple Silicon, Windows 10/11, Ubuntu 22.04/24.04, X11, GNOME Wayland, multiple display scales, multiple monitors, full-screen applications, sleep/wake, startup, and reduced-motion mode.

Wayland behavior and exact host focusing require real-desktop manual verification in addition to automation.

## 18. Delivery Phases

### Phase 0: Cross-Platform Feasibility

- Tauri global bar on macOS, Windows, Ubuntu X11, and Ubuntu Wayland
- Dynamic circular stove layout
- Detachable bars and multi-monitor positioning
- Topmost behavior and Wayland fallback
- System notifications
- Low-resource file watching

### Phase 1: Local MVP

- Codex, Claude Code, and Pi adapters
- Native session discovery and incremental parsing
- Hooks and Pi extension events
- Harness identity on every burner
- Global and detachable surfaces
- Stove state machine and reliable progress
- Jump-to-origin behavior
- Persistent cooked stoves and manual clearing
- Local notification support

### Phase 2: Remote and IM

- Zero-install SSH source
- Temporary single-file bridge
- Reconnection and remote terminal recovery
- Telegram, Slack, Discord, Lark/Feishu, and generic webhook outputs
- Filters, templates, deduplication, and retries
- WhatsApp feasibility and advanced setup

### Phase 3: Adapter Ecosystem

- External adapter process protocol
- Additional harness adapters
- Teams notification support
- Optional Cookbench Skill/MCP semantic enhancement

## 19. MVP Acceptance Criteria

1. Cookbench installs and runs graphically on macOS, Windows, and Ubuntu.
2. Starting Codex, Claude Code, or Pi in the original tool automatically creates a stove.
3. Every burner always displays its source harness.
4. The global bar displays all active and uncleared stoves simultaneously.
5. Hover shows project, task, state, current activity, and reliable progress.
6. Needs Human and Cooked are distinguished using authoritative events.
7. Click returns to the original host surface or provides a clear fallback.
8. Cooked stoves remain until manually cleared.
9. Global and detachable stove surfaces can coexist.
10. Cookbench does not copy complete conversations or interfere with harness execution.
11. SSH disconnects never report completion.
12. External notifications remain outbound-only and are configurable by state and destination.

## 20. Existing Project Reuse Strategy

Cookbench should be a clean Tauri/Rust project rather than a fork of a macOS-first or configuration-management product. Existing repositories still provide valuable, selectively reusable work.

| Project | License | Reuse decision |
| --- | --- | --- |
| [CC Switch](https://github.com/Hortus-Edenensis/cc-switch) | MIT | Reuse cross-platform Tauri packaging, tray, updater, platform configuration, atomic config, test, and release patterns. Do not fork its provider/proxy product domain. |
| [CodeIsland](https://github.com/wxtsky/CodeIsland) | MIT | Port or adapt hook installers, event normalization ideas, session schemas, and adapter fixtures after review. Its Swift overlay UI is a reference, not the cross-platform core. |
| [DevIsland](https://github.com/nangchang/DevIsland) | MIT | Reuse provider-boundary, IPC, terminal-focus, and test-organization ideas. Port only bounded components with attribution. |
| [AgentBar](https://github.com/michalstrnadel/AgentBar) | MIT | Its atomic `state.d` file protocol and hook fallback model are strong references. Reuse only after adding Cookbench-specific schemas and tests. |
| [agent-status](https://github.com/autonomous-ai/agent-status) | Apache-2.0 | Study and potentially port transcript tailing, provider interfaces, and state tests. Any copied code must retain Apache notices and document modifications. |
| [CodexLens](https://github.com/Yukhy/codexlens) | MIT | Reuse Codex read-only discovery and correlation test ideas where they reduce risk. |
| [Claude Status](https://github.com/gmr/claude-status) | BSD-3-Clause | Reference macOS session focus, process correlation, and native notification behavior, preserving BSD attribution for copied code. |
| [AgentHUD](https://github.com/neochoon/agenthud) | No license file found during review | Ideas and observed behavior only. Do not copy source code without explicit permission or a license. |
| [Vibe Kanban](https://github.com/BloopAI/vibe-kanban) | Apache-2.0 | Product and task-model reference only. Its orchestration-first browser product is not an appropriate Cookbench base. |

Assets, mascots, third-party logos, and brand artwork require separate review. A permissive source-code license does not automatically grant trademark rights. Cookbench should use original stove visuals and approved provider marks.

Before copying any source, the implementation plan must record the originating file, license, modifications, and required notice. Prefer independently implementing small concepts when direct reuse would import unnecessary architecture.

## 21. Architecture Decision Summary

- Build a new Tauri 2 application with a shared Rust core and React/TypeScript UI.
- Treat native harness sessions as the source of truth.
- Use hooks/extensions for immediacy and native files for recovery and structured progress.
- Avoid SQLite and full conversation duplication in the first version.
- Keep cooked stove summaries and user layout in small atomic JSON files.
- Support local, zero-install SSH, and optional bridge sources through one event model.
- Keep IM notifications outbound-only and independently configurable.
- Reuse tested infrastructure selectively under explicit license tracking rather than forking an unrelated product.
