//! Tests for handoff domain types.

use super::{HandoffMetadata, HandoffParams, HandoffStatus, ToolCallReference};
use crate::message::domain::{AgentSessionId, MessageId, SequenceNumber, TurnId};
use mockable::DefaultClock;
use rstest::fixture;
use rstest::rstest;

#[fixture]
fn clock() -> DefaultClock {
    DefaultClock
}

#[fixture]
fn handoff_params() -> HandoffParams {
    HandoffParams::new(
        AgentSessionId::new(),
        TurnId::new(),
        "claude-code",
        "opus-agent",
    )
}

#[rstest]
fn handoff_metadata_new_sets_initiated_status(clock: DefaultClock, handoff_params: HandoffParams) {
    let handoff = HandoffMetadata::new(handoff_params, &clock);

    assert_eq!(handoff.status, HandoffStatus::Initiated);
    assert!(handoff.target_session_id.is_none());
    assert!(handoff.completed_at.is_none());
    assert!(handoff.triggering_tool_calls.is_empty());
}

#[rstest]
fn handoff_metadata_complete_sets_target_and_timestamp(
    clock: DefaultClock,
    handoff_params: HandoffParams,
) {
    let handoff = HandoffMetadata::new(handoff_params, &clock);

    let target_session = AgentSessionId::new();
    let completed = handoff.complete(target_session, &clock);

    assert_eq!(completed.status, HandoffStatus::Completed);
    assert_eq!(completed.target_session_id, Some(target_session));
    assert!(completed.completed_at.is_some());
    assert!(completed.is_terminal());
}

#[rstest]
fn handoff_metadata_with_tool_calls_accumulates(
    clock: DefaultClock,
    handoff_params: HandoffParams,
) {
    let msg_id = MessageId::new();
    let seq = SequenceNumber::new(1);

    let handoff = HandoffMetadata::new(handoff_params, &clock)
        .with_triggering_tool_call(ToolCallReference::new("call-1", "read_file", msg_id, seq))
        .with_triggering_tool_call(ToolCallReference::new("call-2", "write_file", msg_id, seq));

    assert_eq!(handoff.triggering_tool_calls.len(), 2);
    assert_eq!(
        handoff
            .triggering_tool_calls
            .first()
            .map(|t| t.call_id.as_str()),
        Some("call-1")
    );
    assert_eq!(
        handoff
            .triggering_tool_calls
            .get(1)
            .map(|t| t.call_id.as_str()),
        Some("call-2")
    );
}

#[rstest]
fn handoff_status_serialization_uses_snake_case() {
    assert_eq!(
        serde_json::to_string(&HandoffStatus::Initiated).expect("serialization"),
        "\"initiated\""
    );
    assert_eq!(
        serde_json::to_string(&HandoffStatus::Completed).expect("serialization"),
        "\"completed\""
    );
}

#[rstest]
fn tool_call_reference_construction() {
    let msg_id = MessageId::new();
    let seq = SequenceNumber::new(42);
    let reference = ToolCallReference::new("call-123", "search", msg_id, seq);

    assert_eq!(reference.call_id, "call-123");
    assert_eq!(reference.tool_name, "search");
    assert_eq!(reference.message_id, msg_id);
    assert_eq!(reference.sequence_number, seq);
}
