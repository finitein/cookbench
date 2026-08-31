# Installing Cookbench

Cookbench is a graphical companion for the Codex, Claude Code, and Pi tools you
already run. It observes their native session files; it does not replace, host,
start, stop, approve, or control those agents.

## One-command Install

Every release publishes small first-party bootstrap scripts beside
`release-manifest.json` and `SHA256SUMS`. The scripts select the matching
platform package, verify its SHA-256 digest, and only then install it.

For a signed stable release:

```bash
# macOS (universal) or graphical Ubuntu/Linux (x86_64)
curl -fsSL https://github.com/finitein/cookbench/releases/latest/download/install.sh | bash
```

```powershell
# Windows PowerShell (x64)
irm https://github.com/finitein/cookbench/releases/latest/download/install.ps1 | iex
```

Preview releases are intentionally opt-in. Pin the tag instead of silently
following the newest preview:

```bash
curl -fsSL https://github.com/finitein/cookbench/releases/download/v0.2.1/install.sh | COOKBENCH_VERSION=v0.2.1 COOKBENCH_ALLOW_PRERELEASE=1 bash
```

```powershell
$env:COOKBENCH_VERSION='v0.2.1'; $env:COOKBENCH_ALLOW_PRERELEASE='1'; irm https://github.com/finitein/cookbench/releases/download/v0.2.1/install.ps1 | iex
```

Use `--dry-run` on macOS/Linux or set `COOKBENCH_DRY_RUN=1` on any platform to
inspect artifact selection without installing it. Unsupported architectures
fail closed rather than downloading a mismatched package.

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

Until then, use the checksum-verifying bootstrap above or a reviewed release
artifact and compare its SHA-256 value with the attached `SHA256SUMS` file.

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
by the user. The compatibility catalog covers 27 harnesses through native
session observation, structured lifecycle hooks, or presence-only experimental
detection. From Settings, Hook Health can automatically manage Cookbench-owned
Codex, Claude Code, Pi, Kimi Code, and ZCode entries; other structured harnesses
show an honest manual-integration state. Hooks write only bounded lifecycle
metadata to the Cookbench spool. Native session files remain authoritative, and
uninstall preserves unrelated harness configuration. The bridge is also
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
