//! Tests for agent session domain types.

use super::{AgentSession, AgentSessionState, HandoffSessionParams};
use crate::message::domain::{ConversationId, HandoffId, SequenceNumber, TurnId};
use mockable::DefaultClock;
use rstest::fixture;
use rstest::rstest;

#[fixture]
fn clock() -> DefaultClock {
    DefaultClock
}

fn assert_reverted_to_active(session: &AgentSession) {
    assert_eq!(session.state, AgentSessionState::Active);
    assert!(session.end_sequence.is_none());
    assert!(session.terminated_by_handoff.is_none());
    assert!(session.ended_at.is_none());
}

fn assert_still_handed_off(session: &AgentSession, expected: HandoffId) {
    assert_eq!(session.state, AgentSessionState::HandedOff);
    assert_eq!(session.terminated_by_handoff, Some(expected));
    assert!(session.end_sequence.is_some());
    assert!(session.ended_at.is_some());
}

#[rstest]
fn agent_session_new_is_active(clock: DefaultClock) {
    let session = AgentSession::new(
        ConversationId::new(),
        "claude-code",
        SequenceNumber::new(1),
        &clock,
    );

    assert_eq!(session.state, AgentSessionState::Active);
    assert!(session.is_active());
    assert!(!session.is_terminal());
    assert!(session.end_sequence.is_none());
    assert!(session.ended_at.is_none());
    assert!(session.initiated_by_handoff.is_none());
}

#[rstest]
fn agent_session_from_handoff(clock: DefaultClock) {
    let handoff_id = HandoffId::new();
    let params = HandoffSessionParams::new(
        ConversationId::new(),
        "opus-agent",
        SequenceNumber::new(10),
        handoff_id,
    );
    let session = AgentSession::from_handoff(params, &clock);

    assert_eq!(session.initiated_by_handoff, Some(handoff_id));
    assert_eq!(session.start_sequence, SequenceNumber::new(10));
}

#[rstest]
fn agent_session_handoff_terminates(clock: DefaultClock) {
    let mut session = AgentSession::new(
        ConversationId::new(),
        "claude-code",
        SequenceNumber::new(1),
        &clock,
    );

    let handoff_id = HandoffId::new();
    session.handoff(SequenceNumber::new(5), handoff_id, &clock);

    assert_eq!(session.state, AgentSessionState::HandedOff);
    assert!(session.is_terminal());
    assert_eq!(session.end_sequence, Some(SequenceNumber::new(5)));
    assert_eq!(session.terminated_by_handoff, Some(handoff_id));
    assert!(session.ended_at.is_some());
}

/// A handed-off session together with its originating handoff
/// ID, ready for revert testing.
struct HandedOffSession {
    session: AgentSession,
    handoff_id: HandoffId,
}

#[fixture]
fn handed_off_session(clock: DefaultClock) -> HandedOffSession {
    let mut session = AgentSession::new(
        ConversationId::new(),
        "claude-code",
        SequenceNumber::new(1),
        &clock,
    );
    let handoff_id = HandoffId::new();
    session.handoff(SequenceNumber::new(5), handoff_id, &clock);
    HandedOffSession {
        session,
        handoff_id,
    }
}

#[rstest]
fn revert_from_handoff_with_matching_id_restores_active(mut handed_off_session: HandedOffSession) {
    let HandedOffSession {
        ref mut session,
        handoff_id,
    } = handed_off_session;

    // Precondition: session is handed off.
    assert_eq!(session.state, AgentSessionState::HandedOff);

    let reverted = session.revert_from_handoff(handoff_id);
    assert!(reverted, "revert should succeed for the matching handoff");
    assert_reverted_to_active(session);
}

#[rstest]
fn revert_from_handoff_with_wrong_id_is_noop(mut handed_off_session: HandedOffSession) {
    let HandedOffSession {
        ref mut session,
        handoff_id,
    } = handed_off_session;

    // Precondition: session is handed off.
    assert_eq!(session.state, AgentSessionState::HandedOff);

    let wrong_id = HandoffId::new();
    let reverted = session.revert_from_handoff(wrong_id);
    assert!(!reverted, "revert should fail for a non-matching handoff");
    assert_still_handed_off(session, handoff_id);
}

#[rstest]
fn agent_session_record_turns(clock: DefaultClock) {
    let mut session = AgentSession::new(
        ConversationId::new(),
        "claude-code",
        SequenceNumber::new(1),
        &clock,
    );

    session.record_turn(TurnId::new());
    session.record_turn(TurnId::new());

    assert_eq!(session.turn_count(), 2);
}

#[rstest]
fn agent_session_state_serialization() {
    assert_eq!(
        serde_json::to_string(&AgentSessionState::Active).expect("serialization"),
        "\"active\""
    );
    assert_eq!(
        serde_json::to_string(&AgentSessionState::HandedOff).expect("serialization"),
        "\"handed_off\""
    );
}
