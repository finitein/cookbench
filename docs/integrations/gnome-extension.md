# GNOME Shell Presentation Extension

The optional `cookbench@cookbench.app` GNOME Shell extension improves the
presentation of Cookbench on GNOME Wayland. It is not required for monitoring,
session discovery, native history recovery, notifications, or normal Tauri UI.
When the extension is absent, disabled, removed, or unable to read its runtime
payload, Cookbench continues as a regular graphical application with the
documented best-effort Wayland overlay behavior.

## Boundary

Cookbench remains authoritative. The application projects its current stove
snapshot into `${XDG_RUNTIME_DIR}/cookbench/gnome-presentation-v1.json` using an
atomic rename. The extension only reads that file and only accepts protocol
version `1`. It does not write state back to Cookbench and exposes no clicks,
commands, agent controls, sockets, listeners, or network access.

The versioned payload allowlist is deliberately small:

```json
{
  "version": 1,
  "revision": 7,
  "stoves": [{
    "harness": "Codex",
    "project": "cookbench",
    "state": "cooking",
    "progress": {"completed": 1, "total": 3},
    "retainedCompletion": false
  }]
}
```

It never contains a native session ID, session path, host or SSH identity,
prompt, transcript, code, command, action text, credential, secret, webhook,
or notification setting. Structured progress is included only when the main
application has already established its provenance. All stoves in every valid
payload are rendered; Cooked remains present until the authoritative main
application clears it.

## Installation

Copy `gnome-extension/` to
`~/.local/share/gnome-shell/extensions/cookbench@cookbench.app/`, then enable it
with GNOME Extensions or:

```bash
gnome-extensions enable cookbench@cookbench.app
```

Disable with `gnome-extensions disable cookbench@cookbench.app`; removal only
removes the presentation helper and leaves Cookbench state and native sessions
untouched. GNOME Shell reload behavior differs by version and session type, so
log out and back in when the Extensions app asks for it.

## Verification Status

`node --test gnome-extension/tests/protocol.test.mjs` is deterministic and
covers protocol version validation, all-stove preservation, and rejection of
session, credential, and notification-setting fields. The current host is macOS
and does not have GNOME Shell or `gjs`, so real extension loading is unverified.

Ubuntu 22.04 GNOME Wayland (Shell 42) and Ubuntu 24.04 GNOME Wayland (Shell 46)
still require manual verification: install, enable, disable, GNOME/session
restart, multiple monitors, extension removal, absent Cookbench behavior, and
atomic payload updates. Those manual checks must also confirm no extension
operation reads harness sessions or alters Cookbench authority.
