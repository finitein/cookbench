# Third-Party Reuse Audit

Date: 2026-08-30

This audit is the gate for incorporating third-party implementation material into
Cookbench. A project appearing here is not permission to copy it. Before source
is copied or ported, the exact originating file, license text, modifications,
and notice obligations must be recorded in this table and in
`THIRD_PARTY_NOTICES.md`.

| Project | Source URL | License | Candidate files or areas | Intended use | Status | Modifications | Required notices |
| --- | --- | --- | --- | --- | --- | --- | --- |
| CC Switch | https://github.com/Hortus-Edenensis/cc-switch | MIT | Tauri build, packaging, tray, updater, and release patterns | Study cross-platform infrastructure patterns without importing its provider/proxy domain | Idea-only | None; no source incorporated | Record exact file and retain MIT copyright/license before any future copy or port |
| CodeIsland | https://github.com/wxtsky/CodeIsland | MIT | Hook installers, event normalization, sanitized fixture design, overlay behavior | Candidate reference for bounded hook and normalization work | Candidate only | None; no source incorporated | Record exact file and retain MIT copyright/license before any future copy or port |
| DevIsland | https://github.com/nangchang/DevIsland | MIT | Provider boundary, IPC, terminal focus, and test organization | Candidate reference for adapter and locator boundaries | Candidate only | None; no source incorporated | Record exact file and retain MIT copyright/license before any future copy or port |
| AgentBar | https://github.com/michalstrnadel/AgentBar | MIT | Atomic `state.d` protocol and hook fallback model | Study small-state and non-invasive fallback ideas | Idea-only | None; no source incorporated | Record exact file and retain MIT copyright/license before any future copy or port |
| agent-status | https://github.com/autonomous-ai/agent-status | Apache-2.0 | Transcript tailing, provider interface, and state tests | Candidate reference for bounded parsing and adapter tests | Candidate only | None; no source incorporated | Preserve license and notices, identify modified files, and mark changes if source is copied or ported |
| CodexLens | https://github.com/Yukhy/codexlens | MIT | Codex read-only discovery and process correlation tests | Candidate reference for the Codex adapter | Candidate only | None; no source incorporated | Record exact file and retain MIT copyright/license before any future copy or port |
| Claude Status | https://github.com/gmr/claude-status | BSD-3-Clause | macOS session focus, process correlation, and native notifications | Candidate reference for macOS locator behavior | Candidate only | None; no source incorporated | Preserve BSD copyright, conditions, and disclaimer before any future copy or port |
| AgentHUD | https://github.com/neochoon/agenthud | No license file found during review | Observed product behavior only | Product idea reference | Idea-only; no-copy | None; source copying and porting are prohibited unless explicit permission or a valid license is obtained | Do not copy source; record future permission or license before reconsideration |
| Vibe Kanban | https://github.com/BloopAI/vibe-kanban | Apache-2.0 | Product and task-model concepts | Product reference only; its orchestration-first architecture is outside Cookbench scope | Idea-only | None; no source incorporated | Record exact file, preserve license/notices, and mark changes before any future copy or port |

## Review Checklist

- Confirm the source URL and license at the revision being evaluated.
- Record exact source files and revision identifiers before copying or porting.
- Prefer independent implementation when the concept is small.
- Document modifications and required attribution in `THIRD_PARTY_NOTICES.md`.
- Review brand artwork, logos, icons, fonts, and other assets separately from code.
- Never copy unlicensed source, real user sessions, prompts, commands, code, or credentials.

