# Security

Cookbench is an observer and presentation layer for existing harness sessions.
It does not start, host, approve, prompt, or otherwise control Codex, Claude
Code, Pi, or another agent.

## Data Boundaries

- Native harness session files remain authoritative. Cookbench does not create a
  conversation database or copy complete transcripts.
- Cookbench-owned persistence is limited to atomic `config.json` and `state.json`
  files. There is no SQLite database in V1.
- Diagnostics are structural only: adapter health, capability flags, parser error
  counts, home-redacted source paths, and enumerated fallback reasons.
- Raw prompts, code, commands, model output, API tokens, passwords, SSH private
  keys, webhook URLs, credential-store values, and notification destinations are
  excluded from diagnostics by type design and redaction tests.
- Session paths under `.ssh` or containing credential, password, secret, token,
  webhook, `id_rsa`, or `id_ed25519` are omitted rather than printed.

## Remote and Notification Boundaries

- Zero-install SSH inspection is read-only. The optional bridge uses SSH stdio,
  opens no listening port, and has no write, prompt, approval, or start-agent
  command.
- Notifications are outbound-only. Cookbench has no inbound IM webhook listener,
  message polling loop, or remote agent-control endpoint.
- Credentials belong in the operating system credential store when notification
  senders are configured. They are not accepted by diagnostics APIs.

## Known Verification Gaps

The deterministic Rust redaction and malformed-path corpus pass on the macOS
development runner. Property fuzzing, package scans, and manual desktop
verification still remain required on Windows, Ubuntu X11, and GNOME Wayland.
