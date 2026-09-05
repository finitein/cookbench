# Verify vs dogfood Vite on :1420

verify.sh runs Playwright on 127.0.0.1:1420 with Vite strictPort.

## Conflict
Dogfood Vite on :1420 makes e2e reuse the wrong server (non-CI) or fail to bind.

## Recipe
1. Stop dogfood Vite on TCP 1420.
2. ss -ltn | grep 1420 should be empty.
3. ./scripts/verify.sh

Release cookbench-desktop does not need Vite. verify.sh warns if :1420 is busy.
