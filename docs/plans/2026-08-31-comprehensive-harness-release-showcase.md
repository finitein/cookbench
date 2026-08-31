# Comprehensive Harness, Release, and Showcase Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Ship an auditable 27-harness compatibility catalog, managed structured-hook support for verified tools, checksum-verified one-command installers and GitHub Release assets, four-language README documentation, and twelve rendered promotional images.

**Architecture:** Preserve the existing adapter trait and `HarnessId::Other` compatibility boundary while adding a declarative catalog and dialect-driven hook projection/configuration. Keep native files authoritative, use hooks only for structured lifecycle and locator identity, and surface capability tiers honestly. Generate offline showcase HTML and deterministic PNGs from shared CSS and Cookbench-owned assets.

**Tech Stack:** Rust 2021, Serde/JSON/TOML already present in the workspace, Tauri 2, React/TypeScript, Node test runner, Vitest, Playwright, POSIX shell, PowerShell, GitHub Actions.

---

### Task 1: Freeze the compatibility catalog contract

**Files:**
- Create: `crates/cookbench-adapters/src/catalog.rs`
- Modify: `crates/cookbench-adapters/src/lib.rs`
- Create: `crates/cookbench-adapters/tests/catalog.rs`

1. Write failing tests for 27 unique stable IDs, required labels/references,
   tier invariants, and verified hook dialects.
2. Run `cargo test -p cookbench-adapters --test catalog` and confirm the module
   is missing.
3. Implement `SupportTier`, `HookDialect`, `ReturnSurface`, `HarnessProfile`,
   `catalog()`, and `profile(id)` using static data only.
4. Re-run the focused test and `cargo fmt --check`.
5. Commit using the Lore Commit Protocol.

### Task 2: Generalize sanitized native-hook ingestion

**Files:**
- Modify: `crates/cookbench-hook/src/main.rs`
- Modify: `crates/cookbench-hook/src/envelope.rs`
- Modify: `src-tauri/src/hook_spool.rs`
- Test: unit tests in both modified Rust modules

1. Add failing tests for Gemini/Qwen/Kimi/Qoder/ZCode/Factory/CodeBuddy/Cursor/
   Copilot/OpenCode/Cline event and identity aliases, unknown harness rejection,
   subagent suppression, and sensitive-content erasure.
2. Run focused package tests and confirm the new cases fail.
3. Replace the three-value helper enum with catalog-allowlisted stable IDs and a
   dialect-driven metadata projection. Map only structured lifecycle events.
4. Accept catalog IDs as `HarnessId::Other`, record per-ID health, and preserve
   bounded locator metadata.
5. Run `cargo test -p cookbench-hook` and relevant `src-tauri` tests; commit.

### Task 3: Add managed hook profiles and health UI

**Files:**
- Modify: `src-tauri/src/hooks/mod.rs`
- Modify: `src/settings/hooks/service.ts`
- Modify: `src/settings/hooks/HookHealthPanel.tsx`
- Modify: `src/settings/hooks/hook-health.css`
- Test: Rust hook tests and `src/settings/hooks/HookHealthPanel.test.tsx`

1. Add failing tests for catalog rows, tier labels, preview-only unsupported
   profiles, JSON merge ownership, backup, repair, uninstall, and conflict
   refusal.
2. Implement data-driven status rows and safe managed writers for verified JSON
   hook dialects; profiles with unverified TOML/plugin writers remain visible
   but non-installable with manual guidance.
3. Add tier/capability presentation without cards nested in cards and keep
   controls accessible.
4. Run Rust and UI focused tests; commit.

### Task 4: Expose expanded source and return metadata

**Files:**
- Modify: `src-tauri/src/runtime/mod.rs`
- Modify: `src-tauri/src/locator/terminal.rs`
- Modify: `src/settings/sources/service.ts`
- Modify: `src/settings/sources/SourcesStatusPanel.tsx`
- Test: runtime, locator, and source-status tests

1. Add failing tests proving hook-only profiles appear in source health, generic
   terminal process correlation is allowlisted, and experimental profiles never
   claim structured completion.
2. Implement catalog-backed source health and process names while retaining
   guarded project/application fallback semantics.
3. Run focused Rust/UI tests and commit.

### Task 5: Build checksum-verified one-command installers

**Files:**
- Create: `scripts/install.sh`
- Create: `scripts/install.ps1`
- Create: `tests/release/installers.test.mjs`
- Modify: `.github/workflows/release.yml`
- Modify: `scripts/release/build-manifest.mjs`
- Modify: `docs/installing.md`

1. Add failing fixture tests for OS/architecture selection, stable/prerelease
   policy, checksum mismatch, dry-run, and unsupported platform errors.
2. Implement shell and PowerShell installers without adding dependencies.
3. Stage both scripts and their hashes in every GitHub Release draft.
4. Run release contract tests, ShellCheck when available, and PowerShell parser
   checks when available; commit.

### Task 6: Rewrite and localize project documentation

**Files:**
- Modify: `README.md`
- Create: `README.zh-CN.md`
- Create: `README.ja.md`
- Create: `README.ko.md`
- Create: `docs/harness-compatibility.md`
- Modify: `docs/releasing.md`

1. Add a documentation contract test that verifies language links, identical
   catalog IDs/count, honest tier wording, one-command installs, security
   boundaries, and showcase links.
2. Rewrite English and author natural Chinese/Japanese/Korean variants rather
   than line-by-line machine-style translations.
3. Run documentation/release tests and link checks; commit.

### Task 7: Author twelve offline showcase pages

**Files:**
- Create: `docs/showcase/shared.css`
- Create: `docs/showcase/assets/cookbench-mark.svg`
- Create: `docs/showcase/01-overview.html` through `12-install.html`
- Create: `docs/showcase/README.md`
- Create: `tests/showcase/showcase.spec.ts`

1. Write a failing Playwright contract that expects all twelve pages, required
   message topics, 1200x1500 rendering, zero overflow, and no remote assets.
2. Implement the shared visual system and twelve standalone documents using
   system fonts, CSS, inline/Cookbench-owned SVG, and real Cookbench UI evidence.
3. Run the focused Playwright test and commit.

### Task 8: Render and visually verify twelve PNGs

**Files:**
- Create: `scripts/render-showcase.mjs`
- Create: `docs/showcase/rendered/01-overview.png` through `12-install.png`
- Modify: `package.json`
- Create: `.omx/state/showcase/ralph-progress.json`

1. Add a failing renderer contract for deterministic filenames and dimensions.
2. Implement the Playwright renderer and capture all PNGs.
3. Inspect every output, run the `visual-verdict` rubric, fix any overlap,
   clipping, weak hierarchy, or inconsistent styling until the score is at
   least 90, and persist the final verdict.
4. Run image dimension/overflow tests and commit the HTML plus rendered assets.

### Task 9: Full verification and release publication

**Files:**
- Modify as required by verification failures
- Update: `docs/verification/release-checklist.md`

1. Run formatting, lint, TypeScript checks, frontend tests, all Rust workspace
   tests, Playwright E2E, release tests, production build, and security fixture
   scans.
2. Inspect the complete diff against the approved design and record real
   platform gaps without converting them into passes.
3. Push the branch, merge or fast-forward the reviewed commits to `main`, tag a
   prerelease, trigger the Release workflow, and wait for every platform job.
4. Verify Release assets, installer checksums, public README language links,
   and all twelve committed PNGs from GitHub.
5. Commit any evidence updates using the Lore Commit Protocol and report the
   public Release URL plus residual signing/platform limitations.
