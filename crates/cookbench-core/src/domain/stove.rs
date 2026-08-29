use serde::{Deserialize, Serialize};

use super::{EventMetadata, ProjectIdentity, StoveIdentity, StructuredProgress};

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum StoveState {
    Starting,
    Planning,
    Cooking,
    NeedsHuman,
    Cooked,
    Failed,
    Disconnected,
    Removed,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct Stove {
    pub identity: StoveIdentity,
    pub project: ProjectIdentity,
    pub state: StoveState,
    pub progress: Option<StructuredProgress>,
    pub state_before_disconnect: Option<StoveState>,
    pub last_event: Option<EventMetadata>,
}

impl Stove {
    pub fn new(identity: StoveIdentity, project: ProjectIdentity) -> Self {
        Self {
            identity,
            project,
            state: StoveState::Starting,
            progress: None,
            state_before_disconnect: None,
            last_event: None,
        }
    }
}
