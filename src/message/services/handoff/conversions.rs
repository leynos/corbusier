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

#[cfg(test)]
mod tests {
    use super::*;
    use rstest::rstest;

    #[rstest]
    #[case(AgentSessionState::Active, HandoffStatus::Initiated)]
    #[case(AgentSessionState::HandedOff, HandoffStatus::Completed)]
    #[case(AgentSessionState::Completed, HandoffStatus::Completed)]
    #[case(AgentSessionState::Failed, HandoffStatus::Failed)]
    #[case(AgentSessionState::Paused, HandoffStatus::Failed)]
    fn agent_session_state_to_handoff_status(
        #[case] state: AgentSessionState,
        #[case] expected: HandoffStatus,
    ) {
        assert_eq!(HandoffStatus::from(state), expected);
    }
}
