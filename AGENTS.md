# Cookbench Agent Contract

This file is the machine-readable front door for Agents working in this
repository. It complements the human-facing `For Agents / 写给 Agent` section in
`README.md`.

## Product Boundary

Cookbench is a lightweight observation and presentation layer for coding-agent
tools. It is not an Agent host, replacement, scheduler, supervisor, or control
plane.

- Native Harness Session files are authoritative.
- One Session maps to one Stove.
- The global Bar shows every visible Stove; detached Stoves may coexist.
- Cooked persists until the user clears it.
- Cookbench must not prompt, approve, start, stop, or otherwise control an Agent.
- Cookbench must not copy full conversations or introduce a SQLite transcript
  store.
- SSH observation is read-only. The optional Bridge uses SSH stdio, opens no
  port, and accepts no remote-control commands.
- Telegram, Slack, Discord, Lark/Feishu, and Generic Webhook integrations are
  outbound-only.

Treat these constraints as product invariants, not implementation details.

## Read Before Editing

Use the narrowest relevant contract:

- `README.md`: product model, installation, architecture, and contributor entry.
- `docs/harness-compatibility.md`: canonical Harness profiles, capability tiers,
  lifecycle evidence, and return surfaces.
- `docs/integrations/hooks.md`: Hook ownership, filtering, install, repair, and
  uninstall behavior.
- `docs/security.md` and `docs/privacy.md`: trust and data boundaries.
- `docs/installing.md`: packaging, SSH, runtime, and removal behavior.
- `docs/verification/release-checklist.md`: 17 acceptance criteria and honest
  platform evidence.
- `docs/plans/2026-08-29-cookbench-design.md`: confirmed product and architecture
  contract.

Prefer existing adapters, reducers, persistence helpers, and UI tokens over new
abstractions or dependencies.

## Data Safety

Never add real user Session content to source, diagnostics, tests, Issues, or
pull requests. Fixtures must be synthetic and metadata-only.

Forbidden fixture and log content includes:

- raw prompts, responses, transcripts, reasoning, or source code;
- commands, tool input/output, environment dumps, or working-tree contents;
- API tokens, passwords, private keys, cookies, credentials, or webhook URLs;
- real usernames, private paths, hostnames, repository names, or Session IDs.

Parse the minimum bounded identity and lifecycle fields required by a feature.
Filter sensitive content at the ingestion boundary rather than after storage.

## State and UI Invariants

- Only Cooking with reliable structured numeric progress may use an incomplete
  progress arc.
- Needs Human, Cooked, Failed, and Disconnected always use complete rings.
- Missing activity is never evidence of Cooked.
- SSH disconnect is Disconnected, never Cooked.
- Parent Harness subagent start/stop events do not create user-facing Stoves.
- Dense Bars wrap into responsive Benches; do not reintroduce horizontal or
  vertical content scrollbars.
- The global Bar and detached Stoves remain movable and freely resizable.
- Hover details are optional and default off.
- Temporary errors expire after 20 seconds.
- Keep the approved lightweight visual language: Cookbench's own SVG marks,
  inline SVG/CSS, and system fonts. Do not add photos, GIFs, video, Lottie,
  sprites, font packages, or third-party logos.

## Adding a Harness

An adapter contributes a normalized observation contract, not a private copy of
the Agent's conversation.

1. Add one stable profile ID and an honest Full, Standard, or Experimental tier
   to the canonical catalog.
2. Discover the documented standard root, or use an explicit absolute path
   supplied by the user.
3. Parse only bounded identity, lifecycle, progress, and locator metadata.
4. Do not infer completion from inactivity or file age.
5. Emit an exact return locator only when it can be uniquely correlated and
   verified. Otherwise provide a guarded app, project, terminal, IDE, or resume
   fallback.
6. Add Cookbench-owned Hook automation only when the Harness exposes a stable
   configuration contract. Preserve unrelated configuration and support clean
   uninstall.
7. Add synthetic fixtures, redaction coverage, lifecycle tests, return tests,
   compatibility documentation, and explicit known gaps.

## Development Workflow

Keep changes scoped, reversible, and consistent with the existing codebase.
Write regression tests before changing behavior that is not already protected.
Do not add a dependency unless the user explicitly requests it and the choice is
documented.

Run focused tests while iterating. Before declaring a release-ready change, run:

```bash
./scripts/verify.sh
```

That gate covers Rust formatting, Clippy with warnings denied, workspace tests
and build, TypeScript checks, Vitest, GNOME protocol tests, Playwright flows,
production build isolation, and source-package audits. Native platform claims
still require the manual evidence listed in the release checklist.

## Lore Commit Protocol

Commit messages are decision records. Use this shape:

```text
<intent line: why the change exists>

<brief narrative context and rationale>

Constraint: <external constraint that shaped the decision>
Rejected: <alternative> | <reason>
Confidence: <low|medium|high>
Scope-risk: <narrow|moderate|broad>
Directive: <warning for future modifiers>
Tested: <verification performed>
Not-tested: <known gaps>
```

The first line states intent rather than restating the diff. Record real test
gaps instead of turning missing native evidence into a claimed pass.
