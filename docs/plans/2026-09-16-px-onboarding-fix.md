# Cookbench PX Onboarding Fix Plan — 2026-09-16

**Branch:** `fix/px-onboarding-empty-settings`  
**Base:** `ad94e3b` / v0.4.4 → **target version 0.4.5**  
**Source report:** `product-experience-reviews/2026-09-15-cookbench-px/report.md`  
**Rollout:** Linux + Windows only (Mac assets / Mac machine out of scope this cycle)

---

## 中文摘要 / English summary

落地报告中的 **PX-01～PX-06**（P1–P2）：空 Bar 可行动 CTA、Settings 可发现性、文档命名对齐 Local Sources、Local Sources 状态语义、Manual Hook 最短下一步、安装叙事指向最新预览 **0.4.5**。不做 Mac 发布、不做 SSH Sources 深改、不做真人可用性测试。

Land **PX-01–PX-06** (P1–P2): actionable empty Bar CTA, Settings discoverability, docs naming → Local Sources, Local Sources status semantics, Manual Hook shortest next step, install narrative → latest preview **0.4.5**. Out of scope: Mac release, SSH Sources deep work, real user testing.

---

## In scope

| ID | Priority | Change |
| --- | --- | --- |
| PX-01 | P1 | Empty Bar CTA → Settings → Local Sources |
| PX-02 | P1 | Always-visible Settings control (incl. minimal) |
| PX-03 | P1 | Docs: Settings → Local Sources / 设置 → 本地来源 |
| PX-04 | P2 | Local Sources: Root monitored ≠ sessions; unavailable hint |
| PX-05 | P2 | Manual Hook: next-step tip + docs link (+ path text) |
| PX-06 | P2 | Install narrative → v0.4.5 Linux/Windows preview |

## Out of scope

- Mac release packaging / Mac machine operations / claiming Mac 0.4.5 assets
- SSH Sources deep UX work
- Real user testing (留待后续招募)
- Force-push, `gh release`, GitHub push (parent handles after report)
- Committing icon noise under `src-tauri/icons/` or whole `product-experience-reviews/` (plan under `docs/plans` only)

---

## 1) PX-01 Empty Bar CTA

**Files:** `GlobalBar.tsx`, `global-bar.css`, `i18n.tsx`, `GlobalBar.test.tsx`, `src/settings/settingsTab.ts`, `NotificationSettingsPanel.tsx`, App wiring via existing `onOpenSettings`.

### Actions

1. Add `src/settings/settingsTab.ts`:
   - `SETTINGS_TAB_KEY = "cookbench.settings.initialTab"`
   - `requestSettingsTab(tab)` / `consumeSettingsTab(): tab | null`
   - Tabs: `"sources" | "hooks" | "general" | "notifications" | "archive"`
   - Persist via `localStorage` so Bar window → Settings window deep-link works.
2. i18n:
   - `bar.emptyHint`: EN `"No sessions to show yet"` / ZH `"还没有可显示的会话"`; ja/ko inherit EN via spread unless overridden — add reasonable overrides in jaComplete/koComplete if desired.
   - New `bar.emptyCta`: EN `"Check sources & hooks"` / ZH `"检查来源与 Hook"` (+ ja/ko).
3. When `stoves.length === 0` && `mode === "full"`: show primary button; `onClick` → `requestSettingsTab("sources")` then `onOpenSettings?.()`.
4. `NotificationSettingsPanel`: on mount `const initial = consumeSettingsTab(); if (initial) setTab(initial);` — default remains `"general"` for tray/gear.
5. Update GlobalBar tests asserting old empty string; add CTA click → request + openSettings.

### Acceptance (report)

- Clean env, no active session: primary CTA one click reaches Local Sources tab.

---

## 2) PX-02 Settings discoverability

**Files:** `GlobalBar.tsx`, `global-bar.css`, `GlobalBar.test.tsx`

### Actions

1. Restyle settings control so it reads as Settings (keep hamburger structure if needed; strengthen contrast / border / hover; ensure `aria-label` = `bar.openSettings` / title = `bar.settings`).
2. Verify minimal + empty brand column does not clip the button (`min-height` / overflow); Settings remains clickable.
3. Test: with `onOpenSettings`, empty **and** minimal modes expose `getByRole("button", { name: /Open Cookbench settings/i })`.

### Acceptance

- Empty state: ≤2 clear clicks into Settings; Settings control always findable without “logo hot zone” folklore.

---

## 3) PX-03 Docs naming

**Files:** `README.md`, `README.zh-CN.md`, `docs/installing.md` (minimal diffs)

### Actions

- Replace **Settings > Sources** → **Settings → Local Sources** (EN).
- ZH: **设置 → 本地来源** (and Hook Health: **设置 → Hook 健康状态** where touched).
- Do not invent Mac 0.4.5 assets.

### Acceptance

- Public docs no longer use obsolete “Settings > Sources” as the current path.

---

## 4) PX-04 Local Sources semantics

**Files:** `SourcesStatusPanel.tsx`, `source-status.css`, `i18n.tsx`, `SourcesStatusPanel.test.tsx`

### Actions

1. Rename `sources.healthy`: `"Root monitored"` / `"监视根目录"`; update ja/ko overrides (`監視ルート` / `루트 감시` or similar).
2. Helper copy:
   - healthy/degraded: `sources.monitoringHint` — monitoring ≠ having sessions (session count already shown).
   - unavailable: `sources.unavailableHint` with actionable copy; `rootDisplay` in title/tooltip.
3. Panel intro: `sources.nextStep` — `"Start a native agent session to light a Stove. Hooks are optional."` / ZH equivalent.
4. Update tests (`Watching` → `Root monitored`, assert nextStep / hints).

### Acceptance

- Untrained reader can tell missing session vs root vs Hook from Local Sources UI.

---

## 5) PX-05 Manual Hook next steps

**Files:** `HookHealthPanel.tsx`, `hook-health.css`, `i18n.tsx`, `HookHealthPanel.test.tsx`

### Actions

1. When `integration === "manual"`: show `hooks.manualNext` tip (hooks optional; start native session first).
2. No opener plugin in tree → show `configDisplay` as selectable text; link **Open Hook docs** to  
   `https://github.com/finitein/cookbench/blob/main/docs/integrations/hooks.md` (`target="_blank" rel="noreferrer"`).
3. Optional “Open config location” only if a real open-path command exists (search found none → skip invoke).
4. Test: manual row shows next-step copy + docs link.

### Acceptance

- Manual rows offer an executable next step without treating Manual as the gate.

---

## 6) PX-06 Install narrative → 0.4.5

**Files:** `README.md`, `README.zh-CN.md`, `docs/installing.md` + version bump files

### Actions

1. Bump **0.4.5** in `package.json`, `src-tauri/tauri.conf.json`, `Cargo.toml` `[workspace.package]` (crates use `version.workspace`).
2. Point “latest preview” / install examples to **v0.4.5** for Linux/Windows; keep unsigned-preview honesty; note Mac follows separately — do **not** claim Mac 0.4.5 packages in this change.
3. Prefer GitHub Releases v0.4.5 primary path for Linux/Windows over pinned v0.4.2 multi-platform narrative.

---

## Tests & verify

1. Focused vitest: GlobalBar, SourcesStatusPanel, HookHealthPanel, NotificationSettings (if touched).
2. Prefer `./scripts/verify.sh` if time; else frontend tests + relevant `cargo test` / `pnpm lint`.
3. Fix failures introduced by this work.

## Linux build (optional)

If verify passes: `pnpm prepare:sidecars` + `pnpm tauri build --config src-tauri/tauri.bundle.conf.json` (or project-equivalent); stage under known path; list SHA256. **No** `gh release` / push.

## Commits

- Local commits only; clear messages; do not commit `icon.icns` noise or icon PNG flood; commit plan + code + docs.

## Remaining for parent

1. Windows smoke on built MSI (when available).
2. Push branch + open PR / release tag v0.4.5.
3. Attach Linux (+ Windows) artifacts; Mac separately later.
4. Optional real-user validation tasks from report §8.

---

## Acceptance checklist (report-aligned)

- [ ] Empty Bar: clear empty copy + CTA → Local Sources in one click after Settings opens
- [ ] Settings control visible in full empty and minimal modes
- [ ] Docs say Local Sources / 本地来源
- [ ] Healthy label = Root monitored / 监视根目录; unavailable has actionable hint; nextStep intro present
- [ ] Manual Hook shows next-step + docs link
- [ ] Version 0.4.5; install narrative points at 0.4.5 Linux/Windows preview
