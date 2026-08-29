use cookbench_core::remote::PollInterval;

use super::ssh::SshError;

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ConnectionState {
    Connected,
    Disconnected { reason: String },
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ReconnectState {
    state: ConnectionState,
    active_sessions: bool,
}

impl Default for ReconnectState {
    fn default() -> Self {
        Self {
            state: ConnectionState::Connected,
            active_sessions: false,
        }
    }
}

impl ReconnectState {
    pub fn state(&self) -> &ConnectionState {
        &self.state
    }

    pub fn record_success(&mut self, active_sessions: bool) -> bool {
        let restored = matches!(self.state, ConnectionState::Disconnected { .. });
        self.state = ConnectionState::Connected;
        self.active_sessions = active_sessions;
        restored
    }

    pub fn record_failure(&mut self, error: &SshError) -> bool {
        let changed = !matches!(self.state, ConnectionState::Disconnected { .. });
        self.state = ConnectionState::Disconnected {
            reason: error.to_string(),
        };
        self.active_sessions = false;
        changed
    }

    pub fn next_interval(&self) -> PollInterval {
        match self.state {
            ConnectionState::Disconnected { .. } => PollInterval::Disconnected,
            ConnectionState::Connected if self.active_sessions => PollInterval::Active,
            ConnectionState::Connected => PollInterval::Idle,
        }
    }
}
