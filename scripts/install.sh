#!/usr/bin/env bash
set -euo pipefail

repository="finitein/cookbench"
manifest_source=""
base_url="https://github.com/${repository}/releases/latest/download"
requested_os=""
requested_arch=""
format="auto"
dry_run=0
allow_prerelease="${COOKBENCH_ALLOW_PRERELEASE:-0}"
version="${COOKBENCH_VERSION:-}"

usage() {
  cat <<'EOF'
Install Cookbench from a checksum-verified GitHub Release.

Usage: install.sh [--version vX.Y.Z] [--allow-prerelease] [--dry-run]
                  [--format auto|appimage|deb]

Environment equivalents: COOKBENCH_VERSION, COOKBENCH_ALLOW_PRERELEASE=1,
COOKBENCH_DRY_RUN=1.
EOF
}

while [[ $# -gt 0 ]]; do
  case "$1" in
    --version) version="${2:?--version requires a tag}"; shift 2 ;;
    --allow-prerelease) allow_prerelease=1; shift ;;
    --dry-run) dry_run=1; shift ;;
    --format) format="${2:?--format requires a value}"; shift 2 ;;
    --manifest) manifest_source="${2:?--manifest requires a path or URL}"; shift 2 ;;
    --base-url) base_url="${2:?--base-url requires a URL}"; shift 2 ;;
    --os) requested_os="${2:?--os requires a value}"; shift 2 ;;
    --arch) requested_arch="${2:?--arch requires a value}"; shift 2 ;;
    --help|-h) usage; exit 0 ;;
    *) echo "Unknown option: $1" >&2; usage >&2; exit 64 ;;
  esac
done

if [[ -n "$version" ]]; then
  [[ "$version" == v* ]] || version="v$version"
  base_url="https://github.com/${repository}/releases/download/${version}"
fi
[[ "${COOKBENCH_DRY_RUN:-0}" == 1 ]] && dry_run=1

case "${requested_os:-$(uname -s)}" in
  Darwin|darwin|macos) os=macos ;;
  Linux|linux) os=linux ;;
  *) echo "Cookbench one-command installation is not available for this operating system." >&2; exit 69 ;;
esac
case "${requested_arch:-$(uname -m)}" in
  arm64|aarch64) arch=arm64 ;;
  x86_64|amd64|x64) arch=x64 ;;
  *) arch="${requested_arch:-$(uname -m)}" ;;
esac

if [[ "$os" == macos ]]; then
  [[ "$arch" == arm64 || "$arch" == x64 ]] || {
    echo "Cookbench is not yet available for ${os}/${arch}." >&2; exit 69;
  }
  suffix="-macos-universal.dmg"
elif [[ "$arch" == x64 ]]; then
  if [[ "$format" == deb ]] || { [[ "$format" == auto ]] && command -v dpkg >/dev/null 2>&1 && [[ "${COOKBENCH_PREFER_DEB:-0}" == 1 ]]; }; then
    suffix="-linux-amd64.deb"
  else
    suffix="-linux-amd64.AppImage"
  fi
else
  echo "Cookbench is not yet available for ${os}/${arch}." >&2
  exit 69
fi

tmp="$(mktemp -d "${TMPDIR:-/tmp}/cookbench-install.XXXXXX")"
cleanup() { rm -rf "$tmp"; }
trap cleanup EXIT
manifest="$tmp/release-manifest.json"

fetch() {
  local source="$1" destination="$2"
  if [[ "$source" =~ ^https?:// ]]; then
    command -v curl >/dev/null 2>&1 || { echo "curl is required." >&2; exit 69; }
    curl --fail --location --silent --show-error "$source" --output "$destination"
  else
    cp "$source" "$destination"
  fi
}

if [[ -n "$manifest_source" ]]; then
  fetch "$manifest_source" "$manifest"
else
  fetch "$base_url/release-manifest.json" "$manifest"
fi

channel="$(awk -F'"' '/"channel"[[:space:]]*:/ { print $4; exit }' "$manifest")"
if [[ -z "$channel" ]]; then
  echo "Release manifest does not declare a channel." >&2
  exit 65
fi
if [[ "$channel" != stable && "$allow_prerelease" != 1 ]]; then
  echo "This Cookbench release is a prerelease and requires --allow-prerelease." >&2
  exit 65
fi

selection="$(awk -v suffix="$suffix" '
  /"name"[[:space:]]*:/ {
    name=$0; sub(/^.*"name"[[:space:]]*:[[:space:]]*"/, "", name); sub(/".*$/, "", name)
    selected=(length(name) >= length(suffix) && substr(name, length(name)-length(suffix)+1) == suffix)
  }
  selected && /"sha256"[[:space:]]*:/ {
    hash=$0; sub(/^.*"sha256"[[:space:]]*:[[:space:]]*"/, "", hash); sub(/".*$/, "", hash)
    print name " " hash; exit
  }
' "$manifest")"
asset="${selection%% *}"
sha256="${selection#* }"
if [[ -z "$selection" || "$asset" == "$sha256" || ! "$sha256" =~ ^[0-9a-fA-F]{64}$ ]]; then
  echo "Release manifest has no valid ${os}/${arch} artifact." >&2
  exit 65
fi

echo "Cookbench artifact: $asset"
echo "SHA-256: $sha256"
if [[ "$dry_run" == 1 ]]; then
  echo "Dry-run: no download or installation was performed."
  exit 0
fi

package="$tmp/$asset"
fetch "$base_url/$asset" "$package"
if command -v shasum >/dev/null 2>&1; then
  actual="$(shasum -a 256 "$package" | awk '{print $1}')"
elif command -v sha256sum >/dev/null 2>&1; then
  actual="$(sha256sum "$package" | awk '{print $1}')"
else
  echo "A SHA-256 utility is required." >&2
  exit 69
fi
actual_lower="$(printf '%s' "$actual" | tr '[:upper:]' '[:lower:]')"
expected_lower="$(printf '%s' "$sha256" | tr '[:upper:]' '[:lower:]')"
[[ "$actual_lower" == "$expected_lower" ]] || { echo "SHA-256 verification failed." >&2; exit 74; }

if [[ "$os" == macos ]]; then
  mount="$(hdiutil attach -nobrowse -readonly "$package" | tail -1 | awk -F'\t' '{print $NF}')"
  app="$mount/Cookbench.app"
  [[ -d "$app" ]] || { hdiutil detach "$mount" >/dev/null; echo "Cookbench.app is missing from the DMG." >&2; exit 65; }
  if [[ -w /Applications ]]; then
    rm -rf /Applications/Cookbench.app
    ditto "$app" /Applications/Cookbench.app
  else
    sudo rm -rf /Applications/Cookbench.app
    sudo ditto "$app" /Applications/Cookbench.app
  fi
  hdiutil detach "$mount" >/dev/null
  open -a Cookbench
elif [[ "$suffix" == *.deb ]]; then
  sudo apt-get install -y "$package"
else
  destination="${HOME}/.local/bin/cookbench"
  mkdir -p "$(dirname "$destination")"
  install -m 0755 "$package" "$destination"
  "$destination" >/dev/null 2>&1 &
fi

echo "Cookbench installation completed."
