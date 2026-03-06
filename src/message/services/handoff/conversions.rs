//! Type conversions between session state and handoff status.

use crate::message::domain::{AgentSessionState, HandoffStatus};

impl From<AgentSessionState> for HandoffStatus {
    fn from(state: AgentSessionState) -> Self {
        match state {
            AgentSessionState::Active => Self::Initiated,
            AgentSessionState::HandedOff | AgentSessionState::Completed => Self::Completed,
            AgentSessionState::Failed | AgentSessionState::Paused => Self::Failed,
        }
    }
}
