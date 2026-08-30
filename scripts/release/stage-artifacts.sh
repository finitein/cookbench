#!/usr/bin/env bash
set -euo pipefail

platform="${1:?platform is required}"
source_root="${2:?bundle root is required}"
output="${3:?output directory is required}"
version="${VERSION:?VERSION is required}"

mkdir -p "$output"

copy_one() {
  local pattern="$1"
  local destination="$2"
  local source
  source="$(find "$source_root" -type f -name "$pattern" -print -quit)"
  if [[ -z "$source" ]]; then
    echo "Missing $pattern under $source_root" >&2
    exit 1
  fi
  cp "$source" "$destination"
}

case "$platform" in
  macos)
    copy_one '*.dmg' "$output/Cookbench-${VERSION}-macos-universal.dmg"
    app="$(find "$source_root" -type d -name 'Cookbench.app' -print -quit)"
    if [[ -z "$app" ]]; then
      echo "Missing Cookbench.app under $source_root" >&2
      exit 1
    fi
    ditto -c -k --sequesterRsrc --keepParent "$app" "$output/Cookbench-${VERSION}-macos-universal.app.zip"
    ;;
  windows)
    copy_one '*.msi' "$output/Cookbench-${VERSION}-windows-x64.msi"
    ;;
  linux)
    copy_one '*.deb' "$output/Cookbench-${VERSION}-linux-amd64.deb"
    copy_one '*.AppImage' "$output/Cookbench-${VERSION}-linux-amd64.AppImage"
    ;;
  *)
    echo "Unknown artifact platform: $platform" >&2
    exit 1
    ;;
esac
