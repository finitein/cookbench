use serde::{Deserialize, Serialize};

/// The application that owned the visible work surface when a session was
/// observed. `Other` is intentionally descriptive only; it does not authorize
/// arbitrary command execution.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum HostApplication {
    MacosTerminal,
    ITerm2,
    WindowsTerminal,
    VisualStudioCode,
    GnomeTerminal,
    Konsole,
    XfceTerminal,
    Other(String),
}

/// A terminal family used only to choose a truthful focus capability.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum TerminalKind {
    Tmux,
    MacosTerminal,
    ITerm2,
    WindowsTerminal,
    GnomeTerminal,
    Konsole,
    XfceTerminal,
    Other(String),
}

/// The smallest useful correlation record for a native session.
///
/// Optional fields reflect information that a harness or host could actually
/// observe. `native_session_id` is an opaque identifier, never session text.
#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub struct SessionLocator {
    pub process_id: Option<u32>,
    pub parent_process_id: Option<u32>,
    pub working_directory: Option<String>,
    pub host_application: Option<HostApplication>,
    pub terminal: Option<TerminalKind>,
    pub tty: Option<String>,
    pub tmux_pane: Option<String>,
    pub ide_workspace: Option<String>,
    pub native_session_id: String,
}

impl SessionLocator {
    pub const MAX_TEXT_BYTES: usize = 4 * 1024;

    /// Reject control characters so locator values can never become shell
    /// source when passed to a platform integration.
    pub fn validate(&self) -> Result<(), LocatorValidationError> {
        if self.native_session_id.is_empty() {
            return Err(LocatorValidationError::EmptyNativeSessionId);
        }

        for value in self.text_values() {
            if value.len() > Self::MAX_TEXT_BYTES || value.chars().any(char::is_control) {
                return Err(LocatorValidationError::UnsafeText);
            }
        }
        Ok(())
    }

    fn text_values(&self) -> impl Iterator<Item = &str> {
        std::iter::once(self.native_session_id.as_str()).chain(
            [
                self.working_directory.as_deref(),
                self.tty.as_deref(),
                self.tmux_pane.as_deref(),
                self.ide_workspace.as_deref(),
            ]
            .into_iter()
            .flatten(),
        )
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LocatorValidationError {
    EmptyNativeSessionId,
    UnsafeText,
}

impl std::fmt::Display for LocatorValidationError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::EmptyNativeSessionId => formatter.write_str("native session ID is required"),
            Self::UnsafeText => {
                formatter.write_str("locator text must be bounded and control-free")
            }
        }
    }
}

impl std::error::Error for LocatorValidationError {}
