# Verify vs dogfood Vite on :1420

verify.sh runs Playwright on 127.0.0.1:1420 with Vite strictPort.

## Conflict
Dogfood Vite on :1420 makes e2e reuse the wrong server (non-CI) or fail to bind.

## Recipe
1. Stop dogfood Vite on TCP 1420.
2. ss -ltn | grep 1420 should be empty.
3. ./scripts/verify.sh

Release cookbench-desktop does not need Vite. verify.sh warns if :1420 is busy.

## Release dogfood (no Vite)

Preferred path matches packaging: use the Tauri CLI release build so frontendDist is embedded.
Enable the cookbench-desktop custom-protocol feature; plain cargo release keeps cfg(dev) and still loads build.devUrl (D18).

Steps:
1. Build via Tauri CLI (`tauri build --no-bundle`) or `cargo build -p cookbench-desktop --release --features custom-protocol` after a frontend production build into dist/.
2. Confirm TCP 1420 is free (no Vite).
3. Launch target/release/cookbench-desktop and confirm the Global Bar UI renders.
