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
    Multiple(Vec<Self>),
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

/// Combines zero or more validation errors into a result.
fn combine_errors(errors: Vec<HandoffValidationError>) -> HandoffValidationResult<()> {
    if errors.is_empty() {
        Ok(())
    } else {
        Err(HandoffValidationError::multiple(errors))
    }
}

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
    /// If the list is empty, this triggers a debug assertion in development
    /// builds as a guard against programming errors.
    #[must_use]
    pub fn multiple(mut errors: Vec<Self>) -> Self {
        debug_assert!(
            !errors.is_empty(),
            "multiple() called with empty error list"
        );
        if errors.len() == 1 {
            errors.swap_remove(0)
        } else {
            Self::Multiple(errors)
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
    let errors: Vec<_> = [
        (handoff.status != HandoffStatus::Initiated && handoff.status != HandoffStatus::Accepted)
            .then_some(HandoffValidationError::InvalidHandoffState {
                expected: HandoffStatus::Initiated,
                actual: handoff.status,
            }),
        handoff
            .target_session_id
            .is_some()
            .then_some(HandoffValidationError::TargetSessionAlreadyAssigned),
    ]
    .into_iter()
    .flatten()
    .collect();

    combine_errors(errors)
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
pub const fn validate_handoff_can_cancel(handoff: &HandoffMetadata) -> HandoffValidationResult<()> {
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
    let range = &snapshot.sequence_range;
    let errors: Vec<_> = [
        (range.start.value() > range.end.value()).then_some(
            HandoffValidationError::InvalidSequenceRange {
                start: range.start.value(),
                end: range.end.value(),
            },
        ),
        (snapshot.snapshot_type != expected_type).then_some(
            HandoffValidationError::InvalidSnapshotType {
                expected: expected_type,
                actual: snapshot.snapshot_type,
            },
        ),
    ]
    .into_iter()
    .flatten()
    .collect();

    combine_errors(errors)
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
    let errors: Vec<_> = [
        validate_session_can_initiate_handoff(session).err(),
        validate_target_agent(target_agent, &session.agent_backend).err(),
    ]
    .into_iter()
    .flatten()
    .collect();

    combine_errors(errors)
}
