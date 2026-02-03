//! Validation rules for handoff operations.
//!
//! This module provides validation for handoff-related domain types,
//! ensuring business rule compliance before persistence or execution.

use crate::message::domain::{
    AgentSession, AgentSessionState, ContextWindowSnapshot, HandoffMetadata, HandoffStatus,
    SnapshotType,
};

/// Error type for handoff validation failures.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HandoffValidationError {
    /// The source session is not in a valid state for handoff.
    InvalidSourceSessionState {
        /// The expected session state.
        expected: AgentSessionState,
        /// The actual session state.
        actual: AgentSessionState,
    },
    /// The handoff is not in a valid state for the requested operation.
    InvalidHandoffState {
        /// The expected handoff status.
        expected: HandoffStatus,
        /// The actual handoff status.
        actual: HandoffStatus,
    },
    /// The target agent identifier is empty or invalid.
    InvalidTargetAgent(String),
    /// The source and target agents are the same.
    SameSourceAndTarget(String),
    /// The handoff references a non-existent session.
    SessionNotFound(String),
    /// The handoff already has a target session assigned.
    TargetSessionAlreadyAssigned,
    /// The context snapshot has invalid sequence range.
    InvalidSequenceRange {
        /// The start of the invalid range.
        start: u64,
        /// The end of the invalid range.
        end: u64,
    },
    /// The snapshot type is not appropriate for the operation.
    InvalidSnapshotType {
        /// The expected snapshot type.
        expected: SnapshotType,
        /// The actual snapshot type.
        actual: SnapshotType,
    },
    /// Multiple validation errors occurred.
    Multiple(Vec<HandoffValidationError>),
}

impl std::fmt::Display for HandoffValidationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidSourceSessionState { expected, actual } => {
                write!(
                    f,
                    "source session must be in {expected:?} state, but is {actual:?}"
                )
            }
            Self::InvalidHandoffState { expected, actual } => {
                write!(
                    f,
                    "handoff must be in {expected:?} state, but is {actual:?}"
                )
            }
            Self::InvalidTargetAgent(reason) => {
                write!(f, "invalid target agent: {reason}")
            }
            Self::SameSourceAndTarget(agent) => {
                write!(f, "source and target agent cannot be the same: {agent}")
            }
            Self::SessionNotFound(id) => {
                write!(f, "session not found: {id}")
            }
            Self::TargetSessionAlreadyAssigned => {
                write!(f, "handoff already has a target session assigned")
            }
            Self::InvalidSequenceRange { start, end } => {
                write!(
                    f,
                    "invalid sequence range: start ({start}) must be <= end ({end})"
                )
            }
            Self::InvalidSnapshotType { expected, actual } => {
                write!(f, "expected snapshot type {expected:?}, but got {actual:?}")
            }
            Self::Multiple(errors) => write_multiple_errors(f, errors),
        }
    }
}

impl std::error::Error for HandoffValidationError {}

/// Helper function to format multiple errors without excessive nesting.
fn write_multiple_errors(
    f: &mut std::fmt::Formatter<'_>,
    errors: &[HandoffValidationError],
) -> std::fmt::Result {
    write!(f, "multiple validation errors: ")?;
    for (i, error) in errors.iter().enumerate() {
        if i > 0 {
            write!(f, "; ")?;
        }
        write!(f, "{error}")?;
    }
    Ok(())
}

impl HandoffValidationError {
    /// Creates a multiple error from a list of errors.
    /// If the list has exactly one error, returns that error.
    /// If the list is empty, panics (should not be called with empty list).
    #[must_use]
    pub fn multiple(errors: Vec<Self>) -> Self {
        match errors.len() {
            0 => panic!("multiple() called with empty error list"),
            1 => errors.into_iter().next().unwrap(),
            _ => Self::Multiple(errors),
        }
    }
}

/// Result type for handoff validation.
pub type HandoffValidationResult<T> = Result<T, HandoffValidationError>;

/// Validates that a session is eligible to initiate a handoff.
///
/// # Requirements
///
/// - Session must be in `Active` state
///
/// # Errors
///
/// Returns `HandoffValidationError::InvalidSourceSessionState` if the session
/// is not active.
pub fn validate_session_can_initiate_handoff(
    session: &AgentSession,
) -> HandoffValidationResult<()> {
    if session.state != AgentSessionState::Active {
        return Err(HandoffValidationError::InvalidSourceSessionState {
            expected: AgentSessionState::Active,
            actual: session.state,
        });
    }
    Ok(())
}

/// Validates a target agent identifier.
///
/// # Requirements
///
/// - Target agent cannot be empty or whitespace-only
/// - Target agent cannot be the same as the source agent
///
/// # Errors
///
/// Returns `HandoffValidationError::InvalidTargetAgent` if the target is empty,
/// or `HandoffValidationError::SameSourceAndTarget` if same as source.
pub fn validate_target_agent(
    target_agent: &str,
    source_agent: &str,
) -> HandoffValidationResult<()> {
    let target_trimmed = target_agent.trim();

    if target_trimmed.is_empty() {
        return Err(HandoffValidationError::InvalidTargetAgent(
            "target agent identifier cannot be empty".to_owned(),
        ));
    }

    if target_trimmed == source_agent.trim() {
        return Err(HandoffValidationError::SameSourceAndTarget(
            source_agent.to_owned(),
        ));
    }

    Ok(())
}

/// Validates that a handoff can be completed.
///
/// # Requirements
///
/// - Handoff must be in `Initiated` or `Accepted` state
/// - Handoff must not already have a target session
///
/// # Errors
///
/// Returns appropriate `HandoffValidationError` if validation fails.
pub fn validate_handoff_can_complete(handoff: &HandoffMetadata) -> HandoffValidationResult<()> {
    let mut errors = Vec::new();

    // Check status
    if handoff.status != HandoffStatus::Initiated && handoff.status != HandoffStatus::Accepted {
        errors.push(HandoffValidationError::InvalidHandoffState {
            expected: HandoffStatus::Initiated,
            actual: handoff.status,
        });
    }

    // Check target session not already assigned
    if handoff.target_session_id.is_some() {
        errors.push(HandoffValidationError::TargetSessionAlreadyAssigned);
    }

    if errors.is_empty() {
        Ok(())
    } else {
        Err(HandoffValidationError::multiple(errors))
    }
}

/// Validates that a handoff can be cancelled.
///
/// # Requirements
///
/// - Handoff must not be in a terminal state (Completed, Cancelled, Failed)
///
/// # Errors
///
/// Returns `HandoffValidationError::InvalidHandoffState` if the handoff is terminal.
pub fn validate_handoff_can_cancel(handoff: &HandoffMetadata) -> HandoffValidationResult<()> {
    if handoff.is_terminal() {
        return Err(HandoffValidationError::InvalidHandoffState {
            expected: HandoffStatus::Initiated,
            actual: handoff.status,
        });
    }
    Ok(())
}

/// Validates a context window snapshot for a handoff operation.
///
/// # Requirements
///
/// - Sequence range must be valid (start <= end)
/// - Snapshot type must be appropriate for the operation
///
/// # Errors
///
/// Returns appropriate `HandoffValidationError` if validation fails.
pub fn validate_snapshot_for_handoff(
    snapshot: &ContextWindowSnapshot,
    expected_type: SnapshotType,
) -> HandoffValidationResult<()> {
    let mut errors = Vec::new();

    // Validate sequence range
    let range = &snapshot.sequence_range;
    if range.start.value() > range.end.value() {
        errors.push(HandoffValidationError::InvalidSequenceRange {
            start: range.start.value(),
            end: range.end.value(),
        });
    }

    // Validate snapshot type
    if snapshot.snapshot_type != expected_type {
        errors.push(HandoffValidationError::InvalidSnapshotType {
            expected: expected_type,
            actual: snapshot.snapshot_type,
        });
    }

    if errors.is_empty() {
        Ok(())
    } else {
        Err(HandoffValidationError::multiple(errors))
    }
}

/// Validates the full handoff initiation request.
///
/// Combines multiple validation checks for initiating a handoff.
///
/// # Requirements
///
/// - Session must be active
/// - Target agent must be valid and different from source
///
/// # Errors
///
/// Returns `HandoffValidationError::Multiple` if multiple validations fail.
pub fn validate_handoff_initiation(
    session: &AgentSession,
    target_agent: &str,
) -> HandoffValidationResult<()> {
    let mut errors = Vec::new();

    if let Err(e) = validate_session_can_initiate_handoff(session) {
        errors.push(e);
    }

    if let Err(e) = validate_target_agent(target_agent, &session.agent_backend) {
        errors.push(e);
    }

    if errors.is_empty() {
        Ok(())
    } else {
        Err(HandoffValidationError::multiple(errors))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::message::domain::{
        AgentSessionId, ConversationId, MessageSummary, SequenceNumber, SequenceRange, TurnId,
    };
    use mockable::DefaultClock;
    use rstest::rstest;

    fn create_active_session() -> AgentSession {
        let clock = DefaultClock;
        AgentSession::new(
            ConversationId::new(),
            "source-agent",
            SequenceNumber::new(1),
            &clock,
        )
    }

    fn create_handoff(status: HandoffStatus) -> HandoffMetadata {
        let clock = DefaultClock;
        let mut handoff = HandoffMetadata::new(
            AgentSessionId::new(),
            TurnId::new(),
            "source-agent",
            "target-agent",
            &clock,
        );
        handoff.status = status;
        handoff
    }

    fn create_snapshot(snapshot_type: SnapshotType) -> ContextWindowSnapshot {
        let clock = DefaultClock;
        ContextWindowSnapshot::new(
            ConversationId::new(),
            AgentSessionId::new(),
            SequenceRange::new(SequenceNumber::new(1), SequenceNumber::new(10)),
            MessageSummary::default(),
            snapshot_type,
            &clock,
        )
    }

    // Session validation tests

    #[rstest]
    fn validate_session_can_initiate_handoff_accepts_active() {
        let session = create_active_session();
        assert!(validate_session_can_initiate_handoff(&session).is_ok());
    }

    #[rstest]
    #[case(AgentSessionState::Paused)]
    #[case(AgentSessionState::HandedOff)]
    #[case(AgentSessionState::Completed)]
    #[case(AgentSessionState::Failed)]
    fn validate_session_can_initiate_handoff_rejects_non_active(#[case] state: AgentSessionState) {
        let mut session = create_active_session();
        session.state = state;

        let result = validate_session_can_initiate_handoff(&session);
        assert!(matches!(
            result,
            Err(HandoffValidationError::InvalidSourceSessionState { .. })
        ));
    }

    // Target agent validation tests

    #[rstest]
    fn validate_target_agent_accepts_valid() {
        assert!(validate_target_agent("target-agent", "source-agent").is_ok());
    }

    #[rstest]
    #[case("")]
    #[case("   ")]
    #[case("\t\n")]
    fn validate_target_agent_rejects_empty(#[case] target: &str) {
        let result = validate_target_agent(target, "source-agent");
        assert!(matches!(
            result,
            Err(HandoffValidationError::InvalidTargetAgent(_))
        ));
    }

    #[rstest]
    fn validate_target_agent_rejects_same_as_source() {
        let result = validate_target_agent("same-agent", "same-agent");
        assert!(matches!(
            result,
            Err(HandoffValidationError::SameSourceAndTarget(_))
        ));
    }

    #[rstest]
    fn validate_target_agent_rejects_same_trimmed() {
        let result = validate_target_agent("  same-agent  ", "same-agent");
        assert!(matches!(
            result,
            Err(HandoffValidationError::SameSourceAndTarget(_))
        ));
    }

    // Handoff completion validation tests

    #[rstest]
    fn validate_handoff_can_complete_accepts_initiated() {
        let handoff = create_handoff(HandoffStatus::Initiated);
        assert!(validate_handoff_can_complete(&handoff).is_ok());
    }

    #[rstest]
    fn validate_handoff_can_complete_accepts_accepted() {
        let handoff = create_handoff(HandoffStatus::Accepted);
        assert!(validate_handoff_can_complete(&handoff).is_ok());
    }

    #[rstest]
    #[case(HandoffStatus::Completed)]
    #[case(HandoffStatus::Cancelled)]
    #[case(HandoffStatus::Failed)]
    fn validate_handoff_can_complete_rejects_terminal(#[case] status: HandoffStatus) {
        let handoff = create_handoff(status);
        let result = validate_handoff_can_complete(&handoff);
        assert!(matches!(
            result,
            Err(HandoffValidationError::InvalidHandoffState { .. })
        ));
    }

    #[rstest]
    fn validate_handoff_can_complete_rejects_with_target_session() {
        let mut handoff = create_handoff(HandoffStatus::Initiated);
        handoff.target_session_id = Some(AgentSessionId::new());

        let result = validate_handoff_can_complete(&handoff);
        assert!(matches!(
            result,
            Err(HandoffValidationError::TargetSessionAlreadyAssigned)
        ));
    }

    // Handoff cancellation validation tests

    #[rstest]
    #[case(HandoffStatus::Initiated)]
    #[case(HandoffStatus::Accepted)]
    fn validate_handoff_can_cancel_accepts_non_terminal(#[case] status: HandoffStatus) {
        let handoff = create_handoff(status);
        assert!(validate_handoff_can_cancel(&handoff).is_ok());
    }

    #[rstest]
    #[case(HandoffStatus::Completed)]
    #[case(HandoffStatus::Cancelled)]
    #[case(HandoffStatus::Failed)]
    fn validate_handoff_can_cancel_rejects_terminal(#[case] status: HandoffStatus) {
        let handoff = create_handoff(status);
        let result = validate_handoff_can_cancel(&handoff);
        assert!(matches!(
            result,
            Err(HandoffValidationError::InvalidHandoffState { .. })
        ));
    }

    // Snapshot validation tests

    #[rstest]
    fn validate_snapshot_for_handoff_accepts_matching_type() {
        let snapshot = create_snapshot(SnapshotType::HandoffInitiated);
        assert!(validate_snapshot_for_handoff(&snapshot, SnapshotType::HandoffInitiated).is_ok());
    }

    #[rstest]
    fn validate_snapshot_for_handoff_rejects_wrong_type() {
        let snapshot = create_snapshot(SnapshotType::SessionStart);
        let result = validate_snapshot_for_handoff(&snapshot, SnapshotType::HandoffInitiated);
        assert!(matches!(
            result,
            Err(HandoffValidationError::InvalidSnapshotType { .. })
        ));
    }

    // Combined validation tests

    #[rstest]
    fn validate_handoff_initiation_accepts_valid() {
        let session = create_active_session();
        assert!(validate_handoff_initiation(&session, "target-agent").is_ok());
    }

    #[rstest]
    fn validate_handoff_initiation_rejects_inactive_session() {
        let mut session = create_active_session();
        session.state = AgentSessionState::Paused;

        let result = validate_handoff_initiation(&session, "target-agent");
        assert!(result.is_err());
    }

    #[rstest]
    fn validate_handoff_initiation_rejects_same_agent() {
        let session = create_active_session();
        let result = validate_handoff_initiation(&session, "source-agent");
        assert!(matches!(
            result,
            Err(HandoffValidationError::SameSourceAndTarget(_))
        ));
    }

    #[rstest]
    fn validate_handoff_initiation_collects_multiple_errors() {
        let mut session = create_active_session();
        session.state = AgentSessionState::Paused;

        let result = validate_handoff_initiation(&session, "source-agent");
        assert!(matches!(result, Err(HandoffValidationError::Multiple(_))));
    }

    // Error display tests

    #[rstest]
    fn error_display_invalid_source_state() {
        let error = HandoffValidationError::InvalidSourceSessionState {
            expected: AgentSessionState::Active,
            actual: AgentSessionState::Paused,
        };
        let display = format!("{error}");
        assert!(display.contains("Active"));
        assert!(display.contains("Paused"));
    }

    #[rstest]
    fn error_display_same_source_and_target() {
        let error = HandoffValidationError::SameSourceAndTarget("test-agent".to_owned());
        let display = format!("{error}");
        assert!(display.contains("test-agent"));
    }

    #[rstest]
    fn error_display_multiple() {
        let errors = vec![
            HandoffValidationError::InvalidTargetAgent("empty".to_owned()),
            HandoffValidationError::TargetSessionAlreadyAssigned,
        ];
        let error = HandoffValidationError::multiple(errors);
        let display = format!("{error}");
        assert!(display.contains("multiple"));
    }
}
