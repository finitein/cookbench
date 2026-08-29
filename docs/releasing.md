# Releasing Cookbench

## Required Evidence

Run `./scripts/verify.sh`, then complete every applicable row in
`docs/verification/release-checklist.md`. A browser or compile result is not a
substitute for native manual checks on macOS, Windows 10/11, Ubuntu 22.04/24.04,
X11, and GNOME Wayland. Record untested platforms as gaps rather than passes.

Before packaging, generate icons from the approved SVG master:

```bash
pnpm tauri icon docs/visual-prototype/assets/cookbench-mark.svg
```

`pnpm prepare:sidecars` builds architecture-matched bridge and hook binaries.
Package locally with `pnpm tauri build --config src-tauri/tauri.bundle.conf.json`.
The Tauri bundle treats the helpers as sidecars; `scripts/package-smoke.sh` fails when
they or required installer formats are absent, or when forbidden runtime media
or font assets are present.

## Signing

- macOS: import a Developer ID Application certificate into a temporary CI
  keychain, set the Apple certificate, identity, account, app-specific password,
  and team secrets used by `release.yml`, then notarize and staple both the app
  and DMG. Do not place certificate material in the repository.
- Windows: configure the organization certificate through the protected CI
  signing provider and timestamp the MSI. Do not use a self-signed certificate
  for a public beta artifact.
- Linux: publish checksums for DEB and AppImage artifacts. Document WebKitGTK,
  libayatana-appindicator, libsecret/Secret Service, and FUSE/AppImage runtime
  expectations in the release notes.

The release workflow creates a draft prerelease. A human release owner reviews
signatures, notarization, package-smoke output, SHA-256 checksums, and the honest
platform matrix before publication.

## Permissions and Secrets

Basic overlay presentation needs no macOS Accessibility permission and no
Windows elevation. Optional local notifications use OS notification permission.
Outbound destination credentials stay in Keychain, Credential Manager, or
Secret Service and must be tested only with synthetic sandbox destinations.

## Integration Locations

- Codex and Claude hooks: follow `docs/integrations/hooks.md`; uninstall must
  preserve unrelated hook entries.
- Pi extension: use `integrations/pi/cookbench.ts` from the original Pi tool.
- GNOME presentation extension: follow
  `docs/integrations/gnome-extension.md` and verify clean removal.
- SSH bridge: distribute only the helper built for the package architecture;
  it has no listener and accepts only the versioned read-only stdio protocol.

## Known Manual Gates

Exact terminal-tab focus, Wayland presentation, multi-monitor restoration,
sleep/wake, full-screen behavior, native notification centers, live outbound
sandboxes, and real SSH transport remain manual release gates. Never infer them
from unit tests or Chromium screenshots.
