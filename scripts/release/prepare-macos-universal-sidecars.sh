#!/usr/bin/env bash
set -euo pipefail

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$root"

for target in aarch64-apple-darwin x86_64-apple-darwin; do
  cargo build --locked --release --target "$target" -p cookbench-bridge -p cookbench-hook
done

mkdir -p src-tauri/binaries
for helper in cookbench-bridge cookbench-hook; do
  lipo -create \
    "target/aarch64-apple-darwin/release/$helper" \
    "target/x86_64-apple-darwin/release/$helper" \
    -output "src-tauri/binaries/$helper-universal-apple-darwin"
done
