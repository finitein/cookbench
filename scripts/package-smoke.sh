#!/usr/bin/env bash
set -euo pipefail

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
mode="${1:-}"

forbidden_regex='\.(gif|jpe?g|webp|avif|mp4|mov|webm|lottie|riv|woff2?|ttf|otf)$|(^|/)(sprites?|sprite-sheets?)(/|\.)'

audit_tree() {
  local tree="$1"
  if find "$tree" -type f | tr '[:upper:]' '[:lower:]' | grep -E "$forbidden_regex"; then
    echo "forbidden runtime visual or font asset found under $tree" >&2
    return 1
  fi
}

audit_tree "$root/src"
audit_tree "$root/src-tauri"
audit_tree "$root/gnome-extension"
audit_tree "$root/dist"

runtime_artwork="$(find "$root/src" "$root/src-tauri" "$root/gnome-extension" -type f \( -iname '*.svg' -o -iname '*.png' -o -iname '*.ico' -o -iname '*.icns' \) | sort)"
expected_artwork="$(printf '%s\n' \
  "$root/src-tauri/icons/icon.png" \
  "$root/src/assets/cookbench-mark.svg" \
  "$root/src/assets/cookbench-tray.svg" | sort)"
if [[ "$runtime_artwork" != "$expected_artwork" ]]; then
  printf 'unexpected runtime artwork inventory:\n%s\n' "$runtime_artwork" >&2
  exit 1
fi
cmp "$root/src/assets/cookbench-mark.svg" "$root/docs/visual-prototype/assets/cookbench-mark.svg"
cmp "$root/src/assets/cookbench-tray.svg" "$root/docs/visual-prototype/assets/cookbench-tray.svg"

if [[ "$mode" == "--source-only" ]]; then
  exit 0
fi

bundle_root="${mode:-$root/target/release/bundle}"
if [[ ! -e "$bundle_root" ]]; then
  echo "package bundle path does not exist: $bundle_root" >&2
  exit 1
fi

audit_tree "$bundle_root"

artifact_count="$(find "$bundle_root" \( -type d -name '*.app' -o -type f \( -name '*.dmg' -o -name '*.msi' -o -name '*.deb' -o -name '*.AppImage' \) \) | wc -l | tr -d ' ')"
if [[ "$artifact_count" == "0" ]]; then
  echo "no DMG/app, MSI, DEB, or AppImage artifact found under $bundle_root" >&2
  exit 1
fi

case "$(uname -s)" in
  Darwin)
    while IFS= read -r image; do hdiutil imageinfo "$image" >/dev/null; done < <(find "$bundle_root" -type f -name '*.dmg')
    ;;
  Linux)
    while IFS= read -r package; do dpkg-deb --info "$package" >/dev/null; done < <(find "$bundle_root" -type f -name '*.deb')
    ;;
esac

for helper in cookbench-bridge cookbench-hook; do
  helper_path="$(find "$bundle_root" -type f \( -name "$helper" -o -name "$helper.exe" -o -name "$helper-*" \) -print -quit)"
  if [[ -z "$helper_path" ]]; then
    echo "$helper is missing from packaged artifacts" >&2
    exit 1
  fi
  if version_output="$("$helper_path" --version 2>/dev/null)"; then
    if [[ "$version_output" != "$helper 0.1.0" ]]; then
      echo "$helper has unexpected version metadata: $version_output" >&2
      exit 1
    fi
  elif ! strings "$helper_path" | grep -q '0\.1\.0'; then
    echo "$helper does not carry expected version metadata" >&2
    exit 1
  fi

  target="${COOKBENCH_TARGET:-$bundle_root}"
  architecture="$(file "$helper_path")"
  case "$target" in
    *aarch64-apple-darwin*) expected_architecture='arm64' ;;
    *x86_64-apple-darwin*) expected_architecture='x86_64' ;;
    *x86_64-pc-windows-msvc*) expected_architecture='(x86-64|x86_64)' ;;
    *x86_64-unknown-linux-gnu*) expected_architecture='(x86-64|x86_64)' ;;
    *) expected_architecture='' ;;
  esac
  if [[ -n "$expected_architecture" ]] && ! grep -Eq "$expected_architecture" <<<"$architecture"; then
    echo "$helper architecture does not match $target: $architecture" >&2
    exit 1
  fi
done

echo "package smoke passed for $bundle_root"
