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
