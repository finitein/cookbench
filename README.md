# Cookbench

<p align="center">
  <img src="src/assets/cookbench-mark.svg" width="88" height="88" alt="Cookbench logo">
</p>

Cookbench is a lightweight desktop companion for the coding agents you already
use. It watches their native session files and turns each session into a small
**Stove** so you can see what is active, what needs attention, and what has
finished without replacing or controlling the original tool.

Cookbench is local-first and pre-release. The current build supports **Codex**,
**Claude Code**, and **Pi** on macOS, Windows, and graphical Ubuntu/Linux.

![Cookbench global bar showing grouped session Stoves](docs/verification/evidence/e2e-grouped-benches.png)

## What It Does

- Shows every discovered session as one Stove in a compact, always-available Bar.
- Separates busy harnesses into benches while keeping small mixed workloads on
  one row.
- Presents Starting, Planning, Cooking, Needs Human, Cooked, Failed, and
  Disconnected states without guessing completion from inactivity.
- Keeps a completed Stove flashing until you click it, while reduced-motion
  mode uses a static emphasis instead.
- Lets you move and resize the global Bar, or detach an individual Stove into
  its own movable window.
- Pins important sessions beyond the normal 48-hour discovery window and keeps
  expired or manually removed sessions in a restorable Archive.
- Returns to a captured terminal pane or Codex Desktop task when a verified
  locator is available, then falls back visibly and safely when it is not.
- Watches remote sessions over read-only SSH, either with no remote install or
  with an optional single-file stdio bridge.
- Sends optional local alerts and outbound-only Telegram, Slack, Discord,
  Lark/Feishu, or generic webhook notifications.

Cookbench never hosts an agent, sends it prompts, approves tool calls, or
deletes its native session history.

## Supported Surfaces

| Area | Current support |
| --- | --- |
| Harnesses | Codex CLI/Desktop, Claude Code, Pi |
| Desktop | macOS, Windows, Ubuntu/Linux X11; GNOME Wayland is best effort with an optional presentation extension |
| Return targets | Codex Desktop, Terminal.app, iTerm2, Ghostty, WezTerm, tmux, Zellij, cmux, plus guarded application/project fallbacks |
| Remote | System SSH read-only discovery; optional verified bridge over SSH stdio only |
| Notifications | Sound, system banner, Bar flash, system attention, and outbound IM/webhooks |

Exact return capability depends on what identity the harness and terminal expose.
Cookbench reports fallbacks rather than claiming a precise jump it cannot verify.
See [session focus verification](docs/verification/session-focus.md) for the
current capability matrix.

## Install

Cookbench is not yet published to Homebrew, winget, or an APT repository. Until
signed release packages are published, build it from source:

```bash
git clone https://github.com/finitein/cookbench.git
cd cookbench
corepack enable
pnpm install
pnpm tauri build
```

The packaged application is written under `src-tauri/target/release/bundle/`.
You need a stable Rust toolchain, Node.js, pnpm, and the
[Tauri 2 platform prerequisites](https://v2.tauri.app/start/prerequisites/) for
your operating system.

For a development build:

```bash
pnpm tauri dev
```

Planned one-command installs, package trust requirements, and platform notes are
documented in [Installing Cookbench](docs/installing.md).

## Getting Started

1. Start Cookbench and then use Codex, Claude Code, or Pi normally.
2. Cookbench automatically scans the standard native session roots from the
   last 48 hours. It does not copy complete conversations.
3. Open **Settings > Sources** to inspect discovery health. Optional managed
   hooks improve lifecycle and terminal identity without changing unrelated
   harness configuration.
4. Click a Stove to return to its original work surface. Use Pin to keep it,
   Delete to archive a non-completed Stove, or Clear to remove a retained Cooked
   Stove from Cookbench only.

Hover details are disabled by default and can be enabled in Settings. Local
alerts default to sound only; banners, Bar flashing, and system attention are
opt-in.

## SSH Sessions

Add a host in Settings using an alias already defined in `~/.ssh/config`, such
as `workstation`. Leave **Session roots** empty to scan the standard Codex,
Claude Code, and Pi roots automatically. Enter explicit absolute roots only for
nonstandard remote layouts.

Cookbench uses the system `ssh` client and existing `known_hosts`. It stores no
SSH password, opens no port, and does not control the remote agent. More detail
is available in [Installing Cookbench](docs/installing.md#native-sources-and-helpers).

## Privacy And Security

Native session files are the source of truth. Cookbench stores only bounded
configuration, sanitized presentation metadata, pin/archive state, and minimum
locators. There is no conversation database, cloud account, telemetry service,
or inbound notification control plane.

Read the full [privacy](docs/privacy.md) and [security](docs/security.md)
boundaries before enabling remote sources or outbound notifications.

## Development

The repository is a Rust workspace with a Tauri 2 desktop shell and a React
frontend.

```bash
# Frontend tests and type checking
pnpm test --run
pnpm lint
pnpm build

# Rust verification
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```

Browser-level visual checks use Playwright:

```bash
pnpm test:e2e
```

Fixtures must remain synthetic and metadata-only. Do not commit real prompts,
session transcripts, code, commands, credentials, keys, or webhook endpoints.

## Project Status

Cookbench is under active development. macOS and Ubuntu X11 have recorded
native verification; Windows and GNOME Wayland still have explicit manual test
gaps. Public package registries, signing, notarization, and auto-update delivery
are not yet live. The repository does not present draft artifacts as production
releases.

Verification evidence and remaining platform gaps are tracked in
[the release checklist](docs/verification/release-checklist.md) and
[the overlay matrix](docs/verification/platform-overlay.md).

## License

Cookbench is available under the [MIT License](LICENSE). Third-party notices are
recorded in [THIRD_PARTY_NOTICES.md](THIRD_PARTY_NOTICES.md).
