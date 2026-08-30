#!/usr/bin/env bash
set -euo pipefail

channel="${RELEASE_CHANNEL:-prerelease}"

case "$channel" in
  prerelease)
    echo "Release channel: prerelease (unsigned artifacts must remain draft/prerelease)."
    ;;
  stable)
    required=(
      APPLE_CERTIFICATE
      APPLE_CERTIFICATE_PASSWORD
      APPLE_SIGNING_IDENTITY
      APPLE_ID
      APPLE_PASSWORD
      APPLE_TEAM_ID
      WINDOWS_SIGNING_CERTIFICATE
      WINDOWS_SIGNING_CERTIFICATE_PASSWORD
      WINDOWS_SIGNING_TIMESTAMP_URL
    )
    missing=()
    for key in "${required[@]}"; do
      if [[ -z "${!key:-}" ]]; then
        missing+=("$key")
      fi
    done

    if (( ${#missing[@]} > 0 )); then
      printf 'Stable releases require signing and notarization credentials. Missing: %s\n' "${missing[*]}" >&2
      exit 1
    fi
    echo "Release channel: stable (macOS signing/notarization and Windows signing are required)."
    ;;
  *)
    echo "Unsupported RELEASE_CHANNEL: $channel (expected prerelease or stable)" >&2
    exit 1
    ;;
esac
