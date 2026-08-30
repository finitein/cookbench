use serde::{Deserialize, Serialize};

/// The application that owned the visible work surface when a session was
/// observed. `Other` is intentionally descriptive only; it does not authorize
/// arbitrary command execution.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum HostApplication {
    /// Codex Desktop accepts an exact native-thread deep-link request. The OS
    /// dispatcher is asynchronous, so focus verification remains best effort.
    CodexDesktop,
    MacosTerminal,
    ITerm2,
    Ghostty,
    WezTerm,
    Zellij,
    Cmux,
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
    Ghostty,
    WezTerm,
    Zellij,
    Cmux,
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
    /// The bounded native-session locator observed from a harness adapter. For
    /// local sessions this is normally the native session file path; it is not
    /// session content and is never executed.
    #[serde(default)]
    pub native_locator: Option<String>,
    #[serde(default)]
    pub process_id: Option<u32>,
    #[serde(default)]
    pub parent_process_id: Option<u32>,
    /// Process start time when a trusted platform observer can supply one.
    /// Together with the PID it prevents stale process identity reuse.
    #[serde(default)]
    pub process_started_at_ms: Option<u64>,
    #[serde(default)]
    pub working_directory: Option<String>,
    #[serde(default)]
    pub host_application: Option<HostApplication>,
    #[serde(default)]
    pub terminal: Option<TerminalKind>,
    #[serde(default)]
    pub tty: Option<String>,
    #[serde(default)]
    pub tmux_pane: Option<String>,
    #[serde(default)]
    pub tmux_inner_pane: Option<String>,
    #[serde(default)]
    pub tmux_outer_client_tty: Option<String>,
    #[serde(default)]
    pub terminal_window_id: Option<String>,
    /// Terminal-family session identity, when the native terminal exposes one
    /// (for example iTerm session ID or Zellij session name).
    #[serde(default)]
    pub terminal_session_id: Option<String>,
    /// Terminal-family pane identity, when exposed by WezTerm, Zellij, cmux,
    /// Ghostty, or another supported terminal.
    #[serde(default)]
    pub terminal_pane_id: Option<String>,
    /// A terminal-owned local control endpoint, for example a WezTerm or cmux
    /// Unix socket path. Credentials are never part of a locator.
    #[serde(default)]
    pub terminal_control_endpoint: Option<String>,
    #[serde(default)]
    pub ide_workspace: Option<String>,
    #[serde(default)]
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
                self.native_locator.as_deref(),
                self.tty.as_deref(),
                self.tmux_pane.as_deref(),
                self.tmux_inner_pane.as_deref(),
                self.tmux_outer_client_tty.as_deref(),
                self.terminal_window_id.as_deref(),
                self.terminal_session_id.as_deref(),
                self.terminal_pane_id.as_deref(),
                self.terminal_control_endpoint.as_deref(),
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
