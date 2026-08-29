# Installing Cookbench

Cookbench is a graphical companion for the Codex, Claude Code, and Pi tools you
already run. It observes their native session files; it does not replace, host,
start, stop, approve, or control those agents.

## macOS

Install the signed DMG for the Mac architecture shown in the release name and
move Cookbench to Applications. Basic Stove presentation does not require
Accessibility permission. macOS may separately ask for notification permission
and Keychain access when those optional features are enabled.

## Windows

Run the signed MSI as a normal user. Cookbench does not require elevation for
its own topmost window. An elevated terminal cannot be focused directly from a
normal-user Cookbench process, so the app falls back to opening the project and
showing resume guidance.

## Ubuntu

Install the DEB or AppImage on a graphical Ubuntu 22.04 or 24.04 desktop. The
WebKitGTK 4.1 runtime and a Secret Service provider are required for the desktop
shell and optional credential storage. X11 supports keep-above behavior. Plain
GNOME Wayland remains best effort; install the optional extension from
`gnome-extension/` by following `docs/integrations/gnome-extension.md` for
panel-level presentation.

## Native Sources and Helpers

Cookbench discovers each harness at its standard session root or a root chosen
by the user. The hook helper is optional and writes bounded lifecycle envelopes
to the Cookbench spool only. The bridge is also optional: after explicit remote
selection, Cookbench uploads one architecture-matched binary, verifies its
SHA-256 digest, sends the selected read-only session roots through the bounded
stdio protocol, and communicates only through SSH standard input/output. The
packaged helper matches the local package platform; select a compatible helper
binary explicitly when the remote platform or architecture differs.

SSH uses the system `ssh` command, the user's existing configuration, and
`known_hosts`. Cookbench stores no SSH password and opens no listening port.

## Removal

Remove the application normally. Uninstall any optional harness hook through
the documented integration command so unrelated user hook configuration is
preserved. Disable and remove the GNOME extension separately if installed.
Native harness sessions are never deleted by Cookbench removal or Stove clear.
