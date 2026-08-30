# Installing Cookbench

Cookbench is a graphical companion for the Codex, Claude Code, and Pi tools you
already run. It observes their native session files; it does not replace, host,
start, stop, approve, or control those agents.

## Registry Status

The commands below are the supported public-install targets, but Cookbench is
**not yet published** to Homebrew, winget, or an APT repository. Do not treat a
draft release artifact as a registry package. A stable release is publishable
only after its signature, notarization where applicable, checksums, and the
generated registry submission metadata have been reviewed and accepted by the
relevant registry or repository host.

Once those submissions are live, the intended commands are:

```bash
brew install --cask cookbench
winget install Cookbench.Cookbench
sudo apt install cookbench
```

Until then, use a reviewed release artifact and compare its SHA-256 value with
the attached `SHA256SUMS` file. There is intentionally no `curl | shell`
bootstrap command.

## macOS

Install the signed universal DMG and move Cookbench to Applications. Basic Stove presentation does not require
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

For the future APT repository, Cookbench will publish a signed source entry and
archive key alongside the DEB. A generated `Packages`/`Release` pair alone is
not an installable APT repository: it must be signed and hosted before users
add it to `sources.list.d`.

## Native Sources and Helpers

Cookbench discovers each harness at its standard session root or a root chosen
by the user. From Settings, optional Hook Health controls install only
Cookbench-owned Codex, Claude Code, and Pi lifecycle entries and write bounded
envelopes to the Cookbench spool. The native session files remain authoritative,
and uninstall preserves unrelated harness configuration. The bridge is also
optional: after explicit remote selection, Cookbench uploads one
architecture-matched binary, verifies its
SHA-256 digest, sends the selected read-only session roots through the bounded
stdio protocol, and communicates only through SSH standard input/output. The
packaged helper matches the local package platform; select a compatible helper
binary explicitly when the remote platform or architecture differs.

SSH uses the system `ssh` command, the user's existing configuration, and
`known_hosts`. Cookbench stores no SSH password and opens no listening port.
An empty Session roots field selects automatic discovery of every first-party
Harness supported by the installed Cookbench build. Explicit absolute roots
remain available as an override for nonstandard remote layouts.

## Removal

Remove the application normally. Uninstall any optional harness hook through
the documented integration command so unrelated user hook configuration is
preserved. Disable and remove the GNOME extension separately if installed.
Native harness sessions are never deleted by Cookbench removal or Stove clear.
