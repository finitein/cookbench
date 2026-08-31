# macOS Performance Baseline

Recorded 2026-08-30 on macOS 26.3 (Apple M4, arm64) from the unsigned release
bundle built with Rust 1.98.0 and the production Vite bundle. These measurements
are host-specific evidence, not substitutes for Windows and Ubuntu baselines.

| Scenario | Result | Target | Status |
| --- | --- | --- | --- |
| Idle desktop process after 8 seconds | 92,944 KiB RSS, 0.0% sampled CPU | Below 150 MB RSS and 1% CPU | Passed on this host |
| Idle desktop process after 13 seconds | 92,448 KiB RSS, 0.0% sampled CPU | Below 150 MB RSS and 1% CPU | Passed on this host |
| Local macOS arm64 application bundle | 18,284 KiB allocated on disk | Keep the native companion compact | Passed on this host |
| Hook bounded spool self-test | 8 ms | Helper must return promptly | Passed on this host |
| 1,000 historical source paths | Diagnostics remains capped at 32 paths and below 8 KiB | No full in-memory historical load | Structural test passed |
| 30 active Stove scenario | All 30 identifiers retained by the scale fixture | Smooth operation with 30 active stoves | Structural and Chromium rendering tests passed |

The process was launched directly from
`target/aarch64-apple-darwin/release/bundle/macos/Cookbench.app` and sampled
twice with `ps`. The hook result comes from `cookbench-hook --self-test` and
includes parsing plus one atomic bounded spool write.

The application-bundle footprint was recorded again on 2026-08-31 with
`du -sk target/release/bundle/macos/Cookbench.app`. Like RSS, bundle size is
platform- and build-specific; it must not be presented as a universal package
size.

Native local hook-to-Tauri-to-WebView latency, bridge latency over a real SSH
link, and zero-install refresh latency require live original-tool and isolated
remote runs. They remain release gates; parser/reducer or browser-fixture timing
must not be reported as those end-to-end measurements. Windows and Ubuntu CPU
and RSS baselines also remain pending on their respective runners.
