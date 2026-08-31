# Comprehensive Harness, Release, and Showcase Design

**Status:** Approved on 2026-08-31

## Objective

Expand Cookbench from three adapters into an honest, extensible catalog that
matches or exceeds CodeIsland's public 14-tool coverage, add checksum-verified
one-command installation from GitHub Releases, publish Chinese, English,
Japanese, and Korean project documentation, and produce twelve reusable social
media images from committed HTML sources.

Cookbench remains an observation layer. It does not host, prompt, approve,
interrupt, or remotely control any agent.

## Support Contract

Support is capability-based rather than a marketing boolean:

| Tier | User-visible meaning | Required evidence |
| --- | --- | --- |
| Full | Reliable native identity and lifecycle, managed hook health, and an exact or verified return target | Official session/hook contract plus fixture-backed state and locator tests |
| Standard | Reliable identity and lifecycle, but the host exposes only a guarded application/project/terminal return | Official session/hook contract plus fixture-backed state tests |
| Experimental | Cookbench can detect a supported surface or consume an explicitly configured generic hook, but must not infer completion | Official product/process evidence; UI and docs must name the limitation |

Absence of activity is never translated into `Cooked`. Experimental profiles
cannot emit `Needs Human`, `Cooked`, or `Failed` without a structured event.

## Catalog

The first expanded catalog targets 27 surfaces. Capability is resolved at
runtime so an installed product may move from experimental to standard/full
when a healthy managed hook is present.

| Harness | Initial tier | Primary integration |
| --- | --- | --- |
| Codex CLI/Desktop | Full | Existing session parser, notify hook, desktop/terminal locator |
| Claude Code | Full | Existing JSONL parser and lifecycle hooks |
| Pi / Oh My Pi | Full | Existing session parser and extension |
| Gemini CLI | Full | Official command hooks and native session identity |
| Qwen Code | Full | Official command hooks including session, stop, permission, and subagent events |
| Kimi Code CLI | Full | Official TOML hooks and native session store |
| Qoder CLI/IDE | Full | Official command hooks and transcript identity |
| ZCode | Full | Official command hooks and CLI configuration |
| Factory Droid | Full | Official user hooks and session identity |
| CodeBuddy CLI | Full | Official lifecycle hooks |
| Cursor Agent/IDE | Full | Official hooks with conversation and transcript identity |
| GitHub Copilot CLI | Full | Official local lifecycle hooks |
| OpenCode | Full | Official event/plugin surface and session API |
| Cline CLI/IDE | Full | Official task hooks and session storage |
| Trae / Trae CLI | Standard | Structured hooks when present; guarded IDE/terminal return |
| Grok CLI | Standard | Structured generic hook profile; terminal return |
| Goose | Standard | Native session/CLI surface; guarded application/terminal return |
| Aider | Standard | Native history/session metadata; terminal return |
| Kiro CLI/IDE | Standard | Structured hook/session surface when available |
| Amazon Q Developer CLI | Standard | Shell/IDE session surface and guarded return |
| Roo Code | Standard | IDE task surface and guarded VS Code return |
| Continue | Standard | IDE session surface and guarded IDE return |
| Amp | Standard | CLI session surface and terminal return |
| Mistral Vibe | Standard | CLI session surface and terminal return |
| Crush | Standard | CLI session surface and terminal return |
| OpenHands CLI | Standard | CLI session surface and terminal return |
| Tencent WorkBuddy | Experimental | Desktop presence only until a public identity/lifecycle contract exists |

The catalog itself is not proof of runtime support. Each profile exposes
capability flags, a support tier, official reference, default roots, executable
names, hook dialect, and return surface. Settings shows those facts rather than
presenting every row as equally complete.

## Architecture

### Catalog and identifiers

`HarnessId::Other(String)` remains the forward-compatible storage boundary.
A new adapter catalog owns stable wire IDs and presentation metadata. This
avoids a large enum migration and lets future adapters arrive without changing
persistence schemas.

### Managed hooks

`cookbench-hook --harness <id>` accepts allowlisted catalog identifiers. Native
payload projection is dialect-driven and only reads bounded metadata fields:
event name, session/conversation ID, transcript locator, working directory, and
terminal identity inherited from the environment. Prompt, response, command,
tool input/output, token, credential, and secret fields are never persisted.

Managed configuration uses structured JSON/TOML editing, preserves unrelated
entries, creates a timestamped backup, supports preview/repair/uninstall, and
refuses ambiguous ownership conflicts. A profile without a verified writer is
listed but cannot advertise automatic installation.

### Discovery

Existing native parsers remain authoritative. New profiles first enter through
structured hooks. A bounded generic session-file adapter is allowed only where
an official stable format is documented; it extracts identity metadata and
structural lifecycle records rather than copying conversation content.

### Installation and releases

GitHub Release assets remain platform-native. `install.sh` and `install.ps1`
resolve the requested/latest tag through GitHub, select the correct OS and
architecture asset from `release-manifest.json`, verify SHA-256, and then hand
the package to the native installer. The scripts support dry-run and explicit
version selection. Unsigned prereleases are clearly rejected by default unless
the user opts into the prerelease channel; stable remains signing-gated.

### Documentation and showcase

`README.md` is the English landing document with language links to
`README.zh-CN.md`, `README.ja.md`, and `README.ko.md`. All four share the same
support matrix and security claims.

`docs/showcase/` contains twelve standalone 1200x1500 HTML compositions and a
shared CSS/asset system. Each page is directly renderable and remains readable
without network access. A Playwright script captures deterministic PNGs at
1200x1500. The set covers:

1. Product overview for AI beginners.
2. One glance across concurrent agents.
3. 27-surface compatibility catalog.
4. Honest three-tier support model.
5. Exact return to terminal/IDE/Codex Desktop.
6. Cross-platform macOS, Windows, and Linux.
7. SSH zero-install and optional stdio bridge.
8. Local-first privacy and no conversation database.
9. Managed Hook installation and health repair.
10. Notifications, archive, pinning, and retained completion.
11. Multi-bench layout for heavy multi-agent users.
12. One-command install and open-source call to action.

The visual language reuses Cookbench's two original SVG marks, system fonts,
CSS, and interface screenshots. It does not include third-party logos, stock
photos, font bundles, animation packages, or invented product screenshots.

## Verification

- Unit tests prove catalog uniqueness, tier/capability invariants, hook
  sanitization, event mapping, config merge/uninstall, and installer selection.
- Integration tests feed synthetic metadata-only fixtures for every managed
  hook dialect and prove no prompt/output content reaches the spool.
- UI tests prove all tiers and health states are distinguishable.
- Release tests exercise Linux/macOS shell selection and Windows PowerShell
  selection without installing packages.
- Playwright renders all twelve pages, asserts exact viewport size and no
  horizontal/vertical overflow, and saves PNG evidence.
- Full lint, typecheck, Rust tests, frontend tests, E2E tests, and release
  contract tests run before publication.

## External Evidence

- CodeIsland publicly lists 14 integrations and distinguishes event/jump
  capability: <https://github.com/wxtsky/CodeIsland>
- Qwen Code hooks: <https://qwenlm.github.io/qwen-code-docs/en/users/features/hooks/>
- Factory Droid hooks: <https://docs.factory.ai/harness/hooks>
- Cline hooks: <https://docs.cline.bot/customization/hooks>
- Qoder hooks: <https://docs.qoder.com/cli/hooks>
- Kimi Code hooks: <https://www.kimi.com/code/docs/en/kimi-code-cli/customization/hooks.html>
- ZCode hooks: <https://zcode.z.ai/en/docs/hooks>
- Cursor hooks: <https://prod.cursor.com/docs/hooks>
- GitHub Copilot hooks: <https://docs.github.com/en/copilot/reference/hooks-reference>
- OpenCode SDK/events: <https://opencode.ai/v2/docs/build/sdk>

## Non-goals

- No remote approval, chat input, or agent control.
- No SQLite or copied transcript database.
- No fake exact jumps based only on title similarity.
- No claim of Full support without a structured identity and lifecycle path.
- No silent overwrite of third-party hook configuration.
- No stable unsigned release.
