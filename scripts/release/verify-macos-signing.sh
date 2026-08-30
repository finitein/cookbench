#!/usr/bin/env bash
set -euo pipefail

bundle_root="${1:?bundle root is required}"

if [[ "${RELEASE_CHANNEL:-prerelease}" != "stable" ]]; then
  echo "Skipping macOS signing verification for prerelease artifacts."
  exit 0
fi

app="$(find "$bundle_root" -type d -name 'Cookbench.app' -print -quit)"
dmg="$(find "$bundle_root" -type f -name '*.dmg' -print -quit)"

if [[ -z "$app" || -z "$dmg" ]]; then
  echo "Stable macOS release is missing its app or DMG." >&2
  exit 1
fi

codesign --verify --deep --strict --verbose=2 "$app"
spctl --assess --type execute --verbose=4 "$app"
xcrun stapler validate "$app"
xcrun stapler validate "$dmg"
