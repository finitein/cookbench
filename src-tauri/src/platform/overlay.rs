use std::fmt;

use tauri::{
    AppHandle, LogicalPosition, Manager, Runtime, WebviewUrl, WebviewWindow, WebviewWindowBuilder,
};

use super::apply_platform_overlay;

const GLOBAL_BAR_LABEL: &str = "main";
const DETACHED_BAR_PREFIX: &str = "stove-";

/// The only window-management operations Cookbench exposes to its domain.
/// None of these methods interact with a harness or terminal process.
pub trait OverlayController {
    fn show_global_bar(&self) -> Result<(), OverlayError>;
    fn show_detached(&self, stove_id: &str) -> Result<(), OverlayError>;
    fn set_always_on_top(&self, enabled: bool) -> Result<(), OverlayError>;
    fn move_to_display(&self, display_id: &str, x: f64, y: f64) -> Result<(), OverlayError>;
}

#[derive(Debug)]
pub enum OverlayError {
    MissingWindow(&'static str),
    InvalidStoveId,
    UnknownDisplay(String),
    UnsupportedPlatform,
    BestEffortWayland,
    Tauri(tauri::Error),
}

impl fmt::Display for OverlayError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingWindow(label) => {
                write!(formatter, "Cookbench window `{label}` is unavailable")
            }
            Self::InvalidStoveId => write!(formatter, "stove id must not be empty"),
            Self::UnknownDisplay(display_id) => {
                write!(formatter, "display `{display_id}` is unavailable")
            }
            Self::UnsupportedPlatform => write!(
                formatter,
                "this platform does not support Cookbench overlays"
            ),
            Self::BestEffortWayland => write!(
                formatter,
                "Wayland presented the bar without a guaranteed overlay"
            ),
            Self::Tauri(error) => write!(formatter, "window operation failed: {error}"),
        }
    }
}

impl std::error::Error for OverlayError {}

impl From<tauri::Error> for OverlayError {
    fn from(error: tauri::Error) -> Self {
        Self::Tauri(error)
    }
}

/// Tauri-backed controller used by the desktop application. It only creates and
/// moves borderless Cookbench windows; the native session files remain owned by
/// their harnesses.
pub struct TauriOverlayController<R: Runtime = tauri::Wry> {
    app: AppHandle<R>,
}

impl<R: Runtime> TauriOverlayController<R> {
    pub fn new(app: AppHandle<R>) -> Self {
        Self { app }
    }

    fn global_window(&self) -> Result<WebviewWindow<R>, OverlayError> {
        self.app
            .get_webview_window(GLOBAL_BAR_LABEL)
            .ok_or(OverlayError::MissingWindow(GLOBAL_BAR_LABEL))
    }

    fn detached_label(stove_id: &str) -> Result<String, OverlayError> {
        if stove_id.trim().is_empty() {
            return Err(OverlayError::InvalidStoveId);
        }

        // Tauri labels must be ASCII. Hex encoding preserves uniqueness without
        // putting user project names or session content into a native title.
        let mut label = String::from(DETACHED_BAR_PREFIX);
        for byte in stove_id.as_bytes() {
            use std::fmt::Write as _;
            write!(&mut label, "{byte:02x}").expect("writing into String cannot fail");
        }
        Ok(label)
    }

    fn detached_window(&self, stove_id: &str) -> Result<WebviewWindow<R>, OverlayError> {
        let label = Self::detached_label(stove_id)?;
        if let Some(window) = self.app.get_webview_window(&label) {
            return Ok(window);
        }

        WebviewWindowBuilder::new(&self.app, label, WebviewUrl::App("index.html".into()))
            .title("Cookbench")
            .decorations(false)
            .resizable(false)
            .skip_taskbar(true)
            .inner_size(360.0, 104.0)
            .visible(false)
            .build()
            .map_err(OverlayError::from)
    }

    fn present(window: &WebviewWindow<R>) -> Result<(), OverlayError> {
        window.show()?;
        match apply_platform_overlay(window) {
            Ok(()) => Ok(()),
            // The graphical window is already visible, while the error tells
            // callers not to represent Wayland placement as guaranteed.
            Err(OverlayError::BestEffortWayland) => Err(OverlayError::BestEffortWayland),
            Err(error) => Err(error),
        }
    }
}

impl<R: Runtime> OverlayController for TauriOverlayController<R> {
    fn show_global_bar(&self) -> Result<(), OverlayError> {
        let window = self.global_window()?;
        Self::present(&window)
    }

    fn show_detached(&self, stove_id: &str) -> Result<(), OverlayError> {
        let window = self.detached_window(stove_id)?;
        Self::present(&window)
    }

    fn set_always_on_top(&self, enabled: bool) -> Result<(), OverlayError> {
        let window = self.global_window()?;
        window.set_always_on_top(enabled)?;
        Ok(())
    }

    fn move_to_display(&self, display_id: &str, x: f64, y: f64) -> Result<(), OverlayError> {
        let window = self.global_window()?;
        let monitor = window
            .available_monitors()?
            .into_iter()
            .find(|monitor| monitor.name().is_some_and(|name| name == display_id))
            .ok_or_else(|| OverlayError::UnknownDisplay(display_id.to_owned()))?;
        let origin = monitor.position();
        let scale = monitor.scale_factor();
        window.set_position(LogicalPosition::new(
            origin.x as f64 / scale + x,
            origin.y as f64 / scale + y,
        ))?;
        Ok(())
    }
}
