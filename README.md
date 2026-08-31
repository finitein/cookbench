# Cookbench

<p align="center">
  <img src="src/assets/cookbench-mark.svg" width="88" height="88" alt="Cookbench logo">
</p>

<p align="center">
  <a href="README.md">English</a> ·
  <a href="README.zh-CN.md">简体中文</a> ·
  <a href="README.ja.md">日本語</a> ·
  <a href="README.ko.md">한국어</a>
</p>

Cookbench is a lightweight desktop companion for the coding agents you already
use. It turns each observed session into a small **Stove**, so one glance tells
you what is running, what needs you, and what has finished. The original agent
stays in charge: Cookbench does not host it, send prompts, approve tools, or
remotely control it.

![Cookbench global Bar showing grouped session Stoves](docs/verification/evidence/e2e-grouped-benches.png)

## Why Cookbench

- See concurrent Codex, Claude Code, Pi, Kimi Code, ZCode, and other supported
  sessions in one compact Bar.
- Return to a verified terminal pane, IDE surface, or Codex Desktop task when
  the host exposes enough identity. Unverified targets use an honest fallback.
- Move and freely resize the Bar, detach a Stove, group busy harnesses into
  separate benches, and show every Stove without scrollbars.
- Keep Cooked sessions until you clear them. Pin long-lived work, and restore
  expired or manually removed sessions from Archive.
- Choose sound, system banner, Bar flash, or system attention. Cooked flashing
  continues until you click that Stove.
- Observe a remote machine through system SSH, with zero remote installation or
  an optional single-file bridge that uses SSH stdio and opens no port.

## 27 Harness Profiles

Cookbench has a capability-based catalog of 27 coding-agent surfaces, more than
a flat “supported” badge can explain. **Full**, **Standard**, and
**Experimental** describe the available identity, lifecycle, and return
contract. They do not pretend that every tool exposes the same features.

| Tier | Included surfaces | Meaning |
| --- | --- | --- |
| Full (14) | Codex, Claude Code, Pi, Gemini CLI, Qwen Code, Kimi Code CLI, Qoder, ZCode, Factory Droid, CodeBuddy, Cursor, GitHub Copilot CLI, OpenCode, Cline | Structured identity and lifecycle contract; exact return only when a verified locator is available |
| Standard (12) | Trae, Grok CLI, Goose, Aider, Kiro, Amazon Q Developer, Roo Code, Continue, Amp, Mistral Vibe, Crush, OpenHands CLI | Structured observation with a guarded app, project, IDE, or terminal return |
| Experimental (1) | Tencent WorkBuddy | Presence-only until a public structured identity and lifecycle contract is available |

Five profiles currently have automatic Cookbench-owned Hook setup: Codex,
Claude Code, Pi, Kimi Code, and ZCode. Other structured profiles are visible in
Hook Health with manual status rather than a fake green check. Read the exact
[compatibility matrix](docs/harness-compatibility.md).

## Install Preview

macOS universal and graphical Ubuntu/Linux x86_64:

```bash
curl -fsSL https://github.com/finitein/cookbench/releases/download/v0.2.1/install.sh | COOKBENCH_VERSION=v0.2.1 COOKBENCH_ALLOW_PRERELEASE=1 bash
```

Windows x64 PowerShell:

```powershell
$env:COOKBENCH_VERSION='v0.2.1'; $env:COOKBENCH_ALLOW_PRERELEASE='1'; irm https://github.com/finitein/cookbench/releases/download/v0.2.1/install.ps1 | iex
```

The bootstrap selects a native package from `release-manifest.json` and verifies
its SHA-256 before installation. Preview packages may be unsigned. Stable and
source-build instructions are in [Installing Cookbench](docs/installing.md).

## Getting Started

1. Start Cookbench, then use your coding agents normally.
2. Leave Session roots empty to scan all standard roots supported by this build.
3. Open **Settings > Sources** and **Settings > Hook Health** to see which
   identity and lifecycle signals are actually healthy.
4. Click a Stove to return to its verified work surface. Pin, archive, restore,
   or clear it without deleting the harness's native session.

Hover details are off by default. Local alerts default to sound only. Temporary
errors disappear after 20 seconds instead of occupying a permanent status row.

## Local-first Boundary

Native session files remain the source of truth. Cookbench stores bounded
presentation metadata, settings, pin/archive state, and minimum return locators.
It has no SQLite conversation database, copies no complete transcript, collects
no telemetry, and exposes no inbound messaging or remote-control API.

Read [Privacy](docs/privacy.md), [Security](docs/security.md), and the
[showcase source and rendered media index](docs/showcase/README.md).

## Development

Cookbench is a Rust workspace with a Tauri 2 desktop shell and React frontend.

```bash
corepack enable
pnpm install
pnpm lint
pnpm test --run
pnpm build
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```

Fixtures must be synthetic and metadata-only. Never commit real prompts,
transcripts, code, commands, credentials, keys, or webhook endpoints.

## Status and License

Cookbench is an open-source preview. macOS and Ubuntu X11 have recorded native
evidence. Windows and GNOME Wayland retain explicit manual verification gaps;
unsigned preview artifacts are not presented as stable packages. See the
[release checklist](docs/verification/release-checklist.md).

Licensed under [MIT](LICENSE). Third-party notices are in
[THIRD_PARTY_NOTICES.md](THIRD_PARTY_NOTICES.md).
