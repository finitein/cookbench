# Grok Build synthetic fixtures

Metadata-only layout mirroring xAI Grok Build session storage under
`~/.grok/sessions/<encoded-cwd>/<session-id>/`.

`updates.jsonl` carries allowlisted ACP `sessionUpdate` lifecycle evidence only
(user/agent turn markers, tool status, plan entry statuses, and `state_update`
with stop reasons). Fixtures intentionally omit prompts, responses, tool
arguments, thoughts, and credentials.
