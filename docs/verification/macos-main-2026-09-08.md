# macOS Main Verification: 2026-09-08

## Scope

- Baseline: GitHub `main`, commit `7279c3eb85756957ef94a57de2773ba5b4a383f0`,
  plus the local reliability and documentation changes described here.
- Host: macOS 26.3, Apple silicon. Grok Build 1.0.13 (5e9a58528b76), stable.
- The initial native test build retained package metadata 0.4.2. The subsequent
  release refresh uses 0.4.3. Native observations below refer to this macOS host;
  the maintainer separately reports Windows/Linux testing in place of CI.
- No real session IDs, titles, paths, transcripts, commands, or credentials are
  included in this evidence or its fixtures.

## Native Format Evidence

The observed Grok summaries use `info.id` and `info.cwd`; internal workers are
marked `session_kind: "subagent"`. Both `events.jsonl` and `updates.jsonl` exist
in the inspected session directories. The lifecycle stream is `updates.jsonl`;
`events.jsonl` also contains unrelated diagnostic events and is not substituted
for the lifecycle stream.

Updates wrap allowed fields in `params.update`. Both `session/update` and
`_x.ai/session/update` occur; the latter carries `turn_completed`. Unix-second
timestamps require conversion to milliseconds. The parser accepts only these
two exact method names and the existing lifecycle allowlist. Internal worker
events, thoughts, text, and task noise remain excluded.

The installed source build discovered two recent parent Grok sessions alongside
the existing Codex Stoves. Internal Grok workers did not become visible Stoves.
Full-to-Minimal-to-Full switching was exercised in the installed application.

In the final build, both Grok parent Stoves reached Cooked from observed
lifecycle records. Each Stove was clicked separately. Both produced the
application-window fallback notice: the host application was requested, but
the exact session was not located. Neither is recorded as an exact jump.
Read-only process ancestry identified macOS Terminal, while one Grok process
held multiple native session files; that evidence cannot identify two distinct
selected conversations. Terminal's selected conversation was not inspected.

The active installation is `/Applications/Cookbench.app`; replaced application
bundles were moved to Trash, not permanently deleted. The initial tested
executable matched the source-build executable's SHA-256:
`62b0477fad86bfd717ca51dba90b312ee23a79a1f6c3c709e4e9cd9a8d213b6e`.
The generated duplicate application bundle was moved into the installation
location rather than left as a second runnable package. User sessions, Hook
configuration, and unrelated workspace files were preserved.

## Reliability Changes

- Bounded summary reads, current and legacy identity fields, conflicting-ID
  rejection, parent-only discovery, and native ACP envelope regression coverage.
- Grok exact terminal return requires unique native-session file evidence.
  Multiple open sessions in a process, unknown candidates, inspection timeouts,
  and exhausted discovery budgets prohibit exact return. Stale exact selectors
  are cleared on fallback; working-directory similarity is not identity proof.
- Custom `GROK_HOME` discovery remains supported, but terminal proof currently
  requires the standard `.grok/sessions` layout and otherwise falls back.
- Full Bar width planning uses the current monitor work area to wrap dense
  benches. It avoids a fit/shrink feedback loop, rebinds observers after mode
  changes, and queues monitor refreshes across asynchronous drag completion.
- The 40-Stove Full/Minimal/Full layout test uses synthetic browser data and
  explicit viewport bounds. It is not evidence of native 40-window behavior or
  unlimited capacity on a finite screen.

[Synthetic 40-Stove evidence](evidence/e2e-forty-full-minimal-full.png)

## Verification Limits

The final `./scripts/verify.sh` run exited successfully with one Rust build job,
`RUST_TEST_THREADS=1`, and the isolated browser test port 1421. It covered Rust
formatting, workspace Clippy with warnings denied, workspace tests and build,
TypeScript checking, 186 Vitest tests in 27 files, three GNOME protocol tests,
24 Playwright flows, the production build, test-driver isolation, and the
source-package audit. Focused Grok adapter tests passed 6/6, Grok observation
tests 4/4, and locator fallback tests 30/30. Independent final diff review found
no unresolved actionable issue.

The default highly parallel Rust run twice exceeded an existing five-second
deadline in `a_restored_old_session_can_join_the_running_observer`. It passes
alone and with serialized Rust tests. Read-only diagnosis found no shared
environment mutation or temporary-directory collision; concurrent native
watcher initialization and background observer startup remain a timing risk.
The timeout was not enlarged to hide the failure. A future improvement should
give observer startup and pinned-path processing an explicit readiness or
acknowledgement contract.

The local Rust toolchain reports a debug-symbol stripping warning because its
`rust-objcopy` cannot resolve `libLLVM.dylib`. Application packaging still
succeeds. This is not a signed or notarized release. A Windows cross-check was
blocked in a dependency by the host's missing Windows SDK/C headers, before
reaching Cookbench code.

## v0.4.3 Release Refresh

After the maintainer authorized publication, version metadata was updated to
0.4.3 and the complete local gate was rerun successfully with the same serial
Rust and isolated-browser settings. The three release-contract tests also
passed. The arm64 App bundle passed the package audit, reported version 0.4.3,
and launched after replacing the initial test build. Its executable SHA-256 is
`cb81f86a200f3a084e01d3d6e049b1f8606eb2590b71873044f4fb15f664a15b`.

The release ZIP passed its archive integrity test. Its SHA-256 is
`eeddce09b2bb06bee259acfbe6e712b01a31868bcb1274dd633a876395954d55`.
Only this newly built Apple-silicon package is included in the release
manifest. The maintainer reports native Windows/Linux testing in lieu of CI;
those platform packages and detailed logs were not supplied for publication.
The release commit requests skipping GitHub push CI for this publication only.
