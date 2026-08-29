# Cookbench Implementation Handoff

Date: 2026-08-30
Status: Ready for a new implementation session

## Purpose

This file is the short, canonical entry point for a new coding session. It does
not replace the approved design or implementation plan; it prevents the next
session from losing decisions made during product alignment.

## Source of Truth

Read these files in order before editing:

1. `AGENTS.md` for repository operating rules.
2. `docs/plans/2026-08-29-cookbench-design.md` for the approved product,
   architecture, behavior, security, platform, and acceptance contract.
3. `docs/visual-prototype/README.md` and
   `docs/visual-prototype/index.html` for the approved Precision Stove identity,
   asset budget, and interactive visual reference.
4. `docs/plans/2026-08-30-cookbench-implementation.md` for the ordered,
   test-driven implementation tasks and exact verification commands.

When wording conflicts, the most recent explicit rule in the approved design and
visual specification wins. Do not silently reinterpret the product.

## Non-Negotiable Product Decisions

- Cookbench extends existing harnesses; it does not replace, launch, host, or
  orchestrate Codex, Claude Code, Pi, or future agents.
- One native Session is one Stove and one task. Corrections and further turns in
  that session stay on the same Stove. A new prompt relights a Cooked Stove.
- The global Bar shows every active or retained Stove and dynamically matches the
  Stove count. It never collapses to only the most urgent Stove.
- Per-Session detached Bars coexist with the global Bar and can be positioned like
  desktop notes.
- Cooked appears on authoritative successful turn completion and remains until
  manually cleared. Clearing Cookbench state never deletes native history.
- Every Stove visibly identifies its source harness.
- Hover/focus exposes exact project, task, state, current action, real progress
  provenance, elapsed time, host, and required human action.
- Clicking returns to the exact originating terminal, IDE, application, tmux pane,
  or SSH surface when possible, then degrades through documented fallbacks.
- Native harness session files remain authoritative. V1 uses atomic
  `config.json` and `state.json`, not SQLite, and never duplicates full chats.
- Initial adapters are Codex, Claude Code, and Pi. Preserve the external adapter
  boundary for Gemini CLI, OpenCode, Cursor, DeepSeek harnesses, OpenClaw, Hermes
  Agent, Grok Build, and other future tools.
- Native file parsing supplies recovery and structured history; optional hooks or
  extensions improve immediacy. Skill/MCP support is optional later enrichment,
  not a baseline dependency.
- Remote hosts support both read-only zero-install SSH inspection and an optional
  temporary single-file bridge over SSH stdio. The bridge opens no port and does
  not control agents. Disconnect means Disconnected, never Cooked.
- Notifications are outbound-only. V1 sends configurable state notifications to
  Telegram, Slack, Discord, Lark/Feishu, and generic webhooks. There is no inbound
  polling, chat, approval, or control path. Users choose which states each
  destination receives and can customize bounded notification templates. Teams
  and official WhatsApp Business support are later work.
- The graphical app supports macOS, Windows, and graphical Ubuntu/Linux. Ubuntu
  ships as one application by default; a GNOME Shell extension is optional for
  stronger Wayland presentation.
- Basic always-on-top presentation should not demand unnecessary privilege.
  macOS Accessibility/Automation is requested only for exact host targeting when
  needed; Windows does not require elevation for its own topmost window.

## Non-Negotiable Visual Decisions

- Use the original Precision Stove direction and open-`C` burner mark.
- Runtime UI uses two small SVG masters, CSS/inline-SVG primitives, and system
  fonts. Do not introduce photos, illustration packs, GIFs, videos, Lottie,
  sprites, canvas textures, web fonts, or network-fetched visual assets.
- Do not bundle or imitate third-party provider logos. Use visible names and
  neutral tokens such as `CX`, `CL`, and `PI`.
- Attention, Cooked, Failed, and Disconnected are complete rings. Their arc
  lengths never encode state.
- Attention is amber and may pulse without rotating. Cooked is green and static
  after one short finish effect. Failed is red and static. Disconnected is gray
  and static.
- Only Cooking with trustworthy structured task progress may show an incomplete
  progress arc and percentage. Cooking without structured progress is
  indeterminate and never invents a percentage.
- Center labels, visible text, tooltips, and accessible names supplement color.
- The Stove itself supplies MVP personality. A standalone mascot illustration is
  deferred and must not increase the runtime package by default.

## Delivery Discipline

- Start with Phase 0 and execute the implementation plan in order.
- Use test-driven development for domain state, adapters, persistence, remote
  transport, notifications, and UI contracts.
- Do not copy third-party source until provenance, license, modifications, and
  notices are recorded. AgentHUD remains idea-only because no license was found.
- Do not commit real prompts, code, commands, tokens, credentials, or native user
  sessions as fixtures.
- Keep the application usable after each task and commit each completed task using
  the repository Lore commit protocol.
- Run the relevant tests after every task and the full lint, test, build, package,
  cross-platform, and visual checks before declaring completion.

## Definition of Done

Implementation is complete only when all 17 acceptance criteria in the approved
design pass, the full verification section of the implementation plan has been
executed, all supported platforms have evidence, third-party notices are current,
and residual Wayland or exact-focus limitations are documented honestly.

## Recommended First Action

Read the four source-of-truth documents, inspect the repository state, and begin
Task 1 of the implementation plan. Do not generate a replacement plan or start
with visual polish; the plan already sequences cross-platform risk before feature
expansion.

## New Session Bootstrap Prompt

```text
你现在负责在当前仓库完整实现 Cookbench。不要重新发明需求，也不要停留在分析或重新规划阶段。

开始前请按顺序完整阅读：
1. AGENTS.md
2. docs/plans/2026-08-30-cookbench-implementation-handoff.md
3. docs/plans/2026-08-29-cookbench-design.md
4. docs/visual-prototype/README.md
5. docs/visual-prototype/index.html
6. docs/plans/2026-08-30-cookbench-implementation.md

这些文件是已经确认的产品、架构、视觉、测试和交付合同。请使用 superpowers:executing-plans 工作流，从实施计划的 Task 1 开始，按照顺序执行。每个任务都应完成代码、测试、验证和符合 AGENTS.md Lore Commit Protocol 的提交，然后继续下一项；除非遇到真正无法自行解决的破坏性决定、凭证限制或外部生产权限，不要请求普通实施许可。

必须特别遵守：
- Cookbench 是 Codex、Claude Code、Pi 等原工具的轻量延伸层，不取代、不托管、不控制 Agent。
- V1 首发 macOS、Windows、图形化 Ubuntu/Linux，并保留 Ubuntu Wayland 的可选 GNOME 扩展。
- 原生 Session 文件是事实来源；不引入 SQLite，不复制完整对话。
- 一个 Session 对应一个 Stove；全局 Bar 展示全部 Stove，独立 Bar 可同时存在；Cooked 保留到用户手动清除。
- 仅有可靠结构化进度的 Cooking 可以显示不完整进度弧。Attention、Cooked、Failed、Disconnected 必须是完整圆环。
- 严格沿用 docs/visual-prototype 的轻量视觉方案：两个小型原创 SVG、CSS/内联 SVG、系统字体；不加入照片、GIF、视频、Lottie、精灵图、字体包或第三方 Logo。
- 首批适配 Codex、Claude Code、Pi，同时保持未来 Harness Adapter 扩展边界。
- SSH 同时支持零安装只读模式和可选单文件 bridge；bridge 只通过 SSH stdio，不开放端口，不控制 Agent。
- IM 仅向外发送状态通知，首批 Telegram、Slack、Discord、Lark/Feishu、Generic Webhook；不接收消息或远程控制。
- 不复制许可证不明确的代码，不把真实用户 Session、提示词、代码、命令、凭证或密钥放进测试夹具。

先简短报告你对当前仓库状态和 Task 1 的理解，然后立即开始实现。持续执行、测试和修复，直到计划完成或出现无法绕过的真实阻塞。最终交付必须逐项核对设计文档中的 17 条验收标准，并提供 macOS、Windows、Ubuntu、远程 SSH、通知、性能、安全和视觉验证证据；无法在当前机器完成的真实平台验证必须明确记录，不得假装通过。
```
