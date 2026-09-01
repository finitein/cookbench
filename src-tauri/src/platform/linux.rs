use std::time::{Duration, Instant};
use tauri::{Runtime, WebviewWindow};

use super::DragReleaseEvidence;

pub(super) fn wait_for_left_release() -> DragReleaseEvidence {
    if is_wayland_session() {
        return DragReleaseEvidence::Unavailable;
    }
    // X11 pointer polling is deliberately bounded to this drag and does not
    // observe coordinates, history, or any other application's input.
    #[repr(C)]
    struct Display {
        _private: [u8; 0],
    }
    #[link(name = "X11")]
    unsafe extern "C" {
        fn XOpenDisplay(name: *const std::ffi::c_char) -> *mut Display;
        fn XCloseDisplay(display: *mut Display) -> i32;
        fn XDefaultRootWindow(display: *mut Display) -> std::ffi::c_ulong;
        fn XQueryPointer(
            display: *mut Display,
            window: std::ffi::c_ulong,
            root: *mut std::ffi::c_ulong,
            child: *mut std::ffi::c_ulong,
            root_x: *mut i32,
            root_y: *mut i32,
            win_x: *mut i32,
            win_y: *mut i32,
            mask: *mut u32,
        ) -> i32;
    }
    // SAFETY: Xlib calls are limited to this worker thread and null checked.
    let display = unsafe { XOpenDisplay(std::ptr::null()) };
    if display.is_null() {
        return DragReleaseEvidence::Unavailable;
    }
    let mut mask = 0;
    let mut root = 0;
    let mut child = 0;
    let mut root_x = 0;
    let mut root_y = 0;
    let mut win_x = 0;
    let mut win_y = 0;
    let mut query = || unsafe {
        let succeeded = XQueryPointer(
            display,
            XDefaultRootWindow(display),
            &mut root,
            &mut child,
            &mut root_x,
            &mut root_y,
            &mut win_x,
            &mut win_y,
            &mut mask,
        );
        (succeeded != 0).then_some(mask & BUTTON1_MASK != 0)
    };
    const BUTTON1_MASK: u32 = 1 << 8;
    let Some(was_down) = query() else {
        unsafe {
            XCloseDisplay(display);
        }
        return DragReleaseEvidence::Unavailable;
    };
    let started = Instant::now();
    let result = if !was_down {
        // The X11 query succeeded after our local drag began; a released
        // Button1 is valid completion evidence for a very short drag.
        super::classify_drag_release(false, false, false)
    } else {
        loop {
            match query() {
                Some(false) => break super::classify_drag_release(true, true, false),
                None => break DragReleaseEvidence::Unavailable,
                Some(true) => {}
            }
            if started.elapsed() >= Duration::from_secs(5) {
                break super::classify_drag_release(true, false, true);
            }
            std::thread::sleep(Duration::from_millis(16));
        }
    };
    // SAFETY: display was returned by XOpenDisplay and is owned here.
    unsafe {
        XCloseDisplay(display);
    }
    result
}

use super::OverlayError;

pub(super) fn apply_overlay<R: Runtime>(window: &WebviewWindow<R>) -> Result<(), OverlayError> {
    if is_wayland_session() {
        // A Wayland compositor can ignore an application's keep-above request.
        // Show the fully functional bar, but never claim overlay success.
        return Err(OverlayError::BestEffortWayland);
    }

    // On X11 Tauri's always-on-top request maps to the window manager's EWMH
    // keep-above hint. The window manager remains the final authority.
    window.set_always_on_top(true)?;
    Ok(())
}

fn is_wayland_session() -> bool {
    std::env::var("XDG_SESSION_TYPE").is_ok_and(|value| value.eq_ignore_ascii_case("wayland"))
        || std::env::var_os("WAYLAND_DISPLAY").is_some()
}
