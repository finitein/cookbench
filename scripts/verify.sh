#!/usr/bin/env bash
set -euo pipefail

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$root"

# Playwright owns 127.0.0.1:1420 (see docs/verification/verify-port-notes.md). Warn when
# a dogfood Vite already holds the port so the gate is not a silent reuse.
if command -v ss >/dev/null 2>&1 && ss -ltn 2>/dev/null | grep -q ':1420 '; then
  echo "warning: :1420 is already in use; Playwright may reuse that server (non-CI)." >&2
  echo "         Free it for a clean gate: fuser -k 1420/tcp   (docs: docs/verification/verify-port-notes.md)" >&2
fi

cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cargo build --workspace
pnpm lint
pnpm test --run
node --test gnome-extension/tests/protocol.test.mjs
pnpm test:e2e
pnpm build

if rg -n '__COOKBENCH_E2E__|CookbenchE2EApp|cookbench-e2e-stoves' dist; then
  echo "test-only E2E driver entered the production build" >&2
  exit 1
fi

"$root/scripts/package-smoke.sh" --source-only
