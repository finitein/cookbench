# Cookbench PX Minimal Empty CTA Plan — 2026-09-16

**Branch:** `fix/px-minimal-empty-cta`  
**Base:** `b2334d1` / v0.4.5 → **target version 0.4.6**  
**Source report:** `product-experience-reviews/2026-09-16-cookbench-px/report.md`  
**Rollout:** Linux + Windows only (Mac assets / Mac machine **out of scope** this cycle)

---

## 中文摘要 / English summary

落地报告 **PX25-01..06**：Minimal 空态迷你 CTA、Mac/安装硬事实叙事（文档）、Settings 可见标签、Local Sources 分组 + Settings Tab 吸顶、CTA/文案对齐 Local Sources、版本升至 **0.4.6**。不做 Mac 打包、不做 live session 自动化、不做完整 glossary 重写、不发 GitHub Release（由 parent 处理）。

Land **PX25-01..06**: Minimal empty mini CTA, hard-fact Mac/install narrative (docs), visible Settings label, Local Sources grouping + sticky Settings tabs, CTA/copy → Local Sources, bump to **0.4.6**. Out of scope: Mac packaging, live-session automation, full glossary rewrite, GitHub release publish (parent).

---

## In scope

| ID | Priority | Change |
| --- | --- | --- |
| PX25-01 | P1 | Minimal empty: hint + mini CTA → Settings → Local Sources; optional expand; Display note |
| PX25-02 | P1 docs | README / installing: hard fact — **no macOS asset on 0.4.6 tag**; pin curl to v0.4.6; Mac → use v0.4.4 (or earlier) Mac preview |
| PX25-04 | P2 | Settings control shows visible `Settings` / `设置` text (keep icon); brand column CSS |
| PX25-05 | P2 | Sticky Settings tab row; Sources list: monitored first, then unavailable (+ optional headers) |
| PX25-06 | P3 | CTA + start-path wording → Open Local Sources / 打开本地来源 |
| Version | — | package.json, tauri.conf.json, Cargo.toml workspace, Cargo.lock → 0.4.6 |

## Out of scope

- Mac packages / Mac machine operations / claiming Mac 0.4.6 assets
- Live Stove session generation automation
- Full glossary rewrite / whole README rewrite
- GitHub `gh release` publish (parent handles after Windows reconnect)
- Committing icon noise under `src-tauri/icons/` or whole `product-experience-reviews/` (plan under `docs/plans` only)

---

## 1) PX25-01 Minimal empty CTA (P1)

**Files:** `GlobalBar.tsx`, `global-bar.css`, `GlobalBar.test.tsx`, `i18n.tsx`, `DisplaySettingsPanel.tsx` (+ test if needed)

### Actions

1. i18n:
   - `bar.emptyCta`: EN `"Open Local Sources"` / ZH `"打开本地来源"` (also ja/ko overrides)
   - `bar.emptyHintMinimal`: EN `"No sessions — next steps"` / ZH `"暂无会话 — 查看下一步"`
   - Keep Full `bar.emptyHint` (`No sessions to show yet` / existing ZH) unless lightly aligning is free
   - `display.modeMinimalNote`: note that Minimal hides the full empty-state checklist until expand **or** mini CTA
2. When `mode === "minimal"` AND `stoves.length === 0`:
   - Primary mini CTA → `requestSettingsTab("sources")` + `onOpenSettings()`
   - Short hint beside/above CTA
   - Optional secondary: existing expand-to-full control (logo / expand)
   - Do **not** only show logo that expands to full
3. Full empty CTA button text uses updated `bar.emptyCta`
4. DisplaySettingsPanel: near Minimal mode radio, show `display.modeMinimalNote`

### Acceptance

- Clean prefs → Minimal start: ≤2 clear clicks reach Local Sources (mini CTA)
- Full empty CTA label is Open Local Sources / 打开本地来源
- Tests: Minimal empty exposes CTA calling settings open; Full empty text updated

---

## 2) PX25-04 Settings visible label (P2)

**Files:** `GlobalBar.tsx`, `global-bar.css`, `GlobalBar.test.tsx`

### Actions

1. Settings button content: icon + visible `t("bar.settings")` (`Settings` / `设置`)
2. Widen brand column / adjust empty+minimal padding so label fits without clip
3. Tests: `getByRole("button", { name: /Open Cookbench settings/i })` still works; Full shows visible Settings text

### Acceptance

- Full (and Minimal if space): user sees Settings text without relying on icon folklore

---

## 3) PX25-05 Sticky tabs + light Sources grouping (P2)

**Files:** `notification-settings.css`, `SourcesStatusPanel.tsx`, `source-status.css`, `i18n.tsx`, `SourcesStatusPanel.test.tsx`

### Actions

1. `.notification-settings__tabs`: `position: sticky; top: 0; z-index` + solid background so Sources/Hooks scroll does not lose tabs
2. `SourcesStatusPanel`: partition list — `healthy`/`degraded` (monitored) first, then `unavailable`
3. Optional section headers: `sources.groupMonitored` / `sources.groupUnavailable` (EN+ZH+ja/ko)
4. Keep changes minimal — no Settings IA rewrite

### Acceptance

- Scrolling Local Sources/Hook content: tab row remains reachable
- Monitored roots appear above unavailable; headers present when both groups non-empty

---

## 4) PX25-02 + education docs (P1 docs)

**Files:** `README.md`, `README.zh-CN.md`, `docs/installing.md`

### Actions

1. Hard-fact banner for preview **0.4.6**: Linux+Windows only; **no macOS asset on this tag**; Mac users use **v0.4.4 (or earlier) Mac preview** until a later tag — concrete, not vague “另行发布”
2. Install curl/PowerShell examples pin **v0.4.6**
3. Getting started: emphasize produce a native agent session first; then empty CTA → Local Sources if still empty
4. Align remaining start-path / CTA “sources” wording to Local Sources
5. Light concept-density trim only if easy (no full rewrite)

### Acceptance

- Stranger Mac user can answer: “0.4.6 has no Mac package; use ≤0.4.4 Mac preview or wait for a later tag”

---

## 5) Version bump 0.4.6

- `package.json`, `src-tauri/tauri.conf.json`, `Cargo.toml` `[workspace.package]`, `Cargo.lock` workspace crate versions as needed

---

## 6) Tests & ship branch

1. `pnpm vitest run` for touched test files + `pnpm lint`
2. Prefer no Rust code changes; `cargo fmt` only if Rust touched
3. Commit with clear messages; `git push -u origin HEAD` (no force)
4. Optional: `gh pr create` if credentials work

---

## Linux + Windows rollout / Mac out of scope

| Platform | This cycle |
| --- | --- |
| Linux | Implement + verify on box; parent may build AppImage |
| Windows | Parent builds/smoke when machine reconnects — **not** this executor |
| macOS | **Out of scope** — no package, no machine, no release claim for 0.4.6 |

---

## Report back checklist

- Files changed, commits, PR URL if any
- Test / lint results
- Remaining Windows steps for parent
