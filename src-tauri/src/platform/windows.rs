use std::time::{Duration, Instant};
use tauri::{Runtime, WebviewWindow};

use super::DragReleaseEvidence;

/// Polls the physical left button only for the bounded lifetime of a native
/// Cookbench drag. GetAsyncKeyState is process-independent and installs no hook.
pub(super) fn wait_for_left_release() -> DragReleaseEvidence {
    const VK_LBUTTON: i32 = 0x01;
    const LIMIT: Duration = Duration::from_secs(5);
    unsafe extern "system" {
        fn GetAsyncKeyState(v_key: i32) -> i16;
    }
    // SAFETY: Win32 documents GetAsyncKeyState as a pure thread-safe query.
    let down = unsafe { GetAsyncKeyState(VK_LBUTTON) } < 0;
    if !down {
        return DragReleaseEvidence::Unavailable;
    }
    let started = Instant::now();
    while started.elapsed() < LIMIT {
        // SAFETY: see the query above; no pointer coordinates are read.
        if unsafe { GetAsyncKeyState(VK_LBUTTON) } >= 0 {
            return DragReleaseEvidence::Released;
        }
        std::thread::sleep(Duration::from_millis(16));
    }
    DragReleaseEvidence::Unavailable
}

use super::OverlayError;

/// A topmost Cookbench-owned window is supported by ordinary Win32 window APIs
/// and does not require elevation.
pub(super) fn apply_overlay<R: Runtime>(window: &WebviewWindow<R>) -> Result<(), OverlayError> {
    window.set_always_on_top(true)?;
    Ok(())
}
