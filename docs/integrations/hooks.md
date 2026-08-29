# Hook Helper

`cookbench-hook` is an optional local lifecycle notifier. It improves update
latency only: native Codex, Claude Code, and Pi session files remain the
authoritative source for recovery and structured history.

The helper reads one JSON object from standard input and writes one atomic JSON
envelope to the existing app-private runtime spool. It uses
`COOKBENCH_HOOK_SPOOL_DIR` when supplied and otherwise resolves Cookbench's
standard per-user app-data directory on macOS, Windows, or Linux.
Cookbench creates and owns that directory; the helper never creates it, waits
for the UI, opens a port, changes a harness configuration, invokes an agent, or
uses the network.

The accepted input has this bounded shape:

```json
{
  "event_type": "tool_started",
  "session_id": "synthetic-session-42",
  "harness": "codex",
  "sequence": 7,
  "progress": { "completed": 1, "total": 3 }
}
```

Supported event types are `session_discovered`, `user_prompt_submitted`,
`plan_updated`, `tool_started`, `tool_completed`, `question_asked`,
`permission_requested`, `turn_completed`, `session_failed`, `process_exited`,
`connection_lost`, and `connection_restored`. Harnesses are `codex`,
`claude_code`, and `pi`.

Input is limited to 16 KiB. The helper rejects unknown fields and fields whose
names imply prompt, output, code, command, token, credential, password, secret,
authorization, or API-key data. It emits only a generic diagnostic and never
echoes input. Session identifiers are ASCII and limited to 256 bytes.

The spool is bounded to 128 finalized envelopes and 1 MiB. Every envelope is
written to a same-directory temporary file with owner-only Unix permissions,
synced, then atomically renamed. A missing spool exits `69`, a full spool exits
`75`, malformed, oversized, sensitive, or unsupported input exits `64`, and an
I/O error exits `74`. Successful delivery exits `0`. Harness integrations should
treat diagnostics as informational so an unavailable Cookbench installation
never interrupts the host workflow.

Run `cookbench-hook --self-test` to exercise a synthetic envelope write and
report its elapsed milliseconds. The command creates no persistent state.
Claude Code passes its native hook JSON over stdin. Codex `notify` passes its
native JSON as the final command argument. The helper extracts only the native
session ID and an allowlisted lifecycle name; transcript paths, prompts, tool
inputs, tool responses, commands, and notification text are discarded before
the spool write. It prints no JSON and returns no control decision to either
tool.
