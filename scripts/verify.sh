#!/usr/bin/env bash
set -euo pipefail

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$root"

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
