//! Tests for handoff validation rules.

use super::handoff::{
    HandoffValidationError, validate_handoff_can_cancel, validate_handoff_can_complete,
    validate_handoff_initiation, validate_session_can_initiate_handoff,
    validate_snapshot_for_handoff, validate_target_agent,
};
use crate::message::domain::{
    AgentSession, AgentSessionId, AgentSessionState, ContextWindowSnapshot, ConversationId,
    HandoffMetadata, HandoffParams, HandoffStatus, MessageSummary, SequenceNumber, SequenceRange,
    SnapshotParams, SnapshotType, TurnId,
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
    let params = HandoffParams::new(
        AgentSessionId::new(),
        TurnId::new(),
        "source-agent",
        "target-agent",
    );
    let mut handoff = HandoffMetadata::new(params, &clock);
    handoff.status = status;
    handoff
}

fn create_snapshot(snapshot_type: SnapshotType) -> ContextWindowSnapshot {
    let clock = DefaultClock;
    let params = SnapshotParams::new(
        ConversationId::new(),
        AgentSessionId::new(),
        SequenceRange::new(SequenceNumber::new(1), SequenceNumber::new(10)),
        MessageSummary::default(),
        snapshot_type,
    );
    ContextWindowSnapshot::new(params, &clock)
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
