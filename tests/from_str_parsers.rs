//! Direct coverage for the `FromStr` parsers of the string-backed domain
//! types: every wire name parses to its variant, and unknown or empty text is
//! refused. Each type is exercised from outside the crate.

use std::str::FromStr;

use corbusier::{
    agent_backend::domain::TurnSessionStatus,
    hook_engine::domain::HookExecutionStatus,
    message::domain::{AgentSessionState, ConversationState, SnapshotType},
    tool_registry::domain::{McpServerHealthStatus, McpServerLifecycleState},
};
use rstest::rstest;

#[rstest]
#[case("active", AgentSessionState::Active)]
#[case("paused", AgentSessionState::Paused)]
#[case("handed_off", AgentSessionState::HandedOff)]
#[case("completed", AgentSessionState::Completed)]
#[case("failed", AgentSessionState::Failed)]
fn agent_session_state_parses_its_wire_names(
    #[case] text: &str,
    #[case] expected: AgentSessionState,
) {
    assert_eq!(AgentSessionState::from_str(text), Ok(expected));
}

#[rstest]
#[case("active", ConversationState::Active)]
#[case("paused", ConversationState::Paused)]
#[case("archived", ConversationState::Archived)]
fn conversation_state_parses_its_wire_names(
    #[case] text: &str,
    #[case] expected: ConversationState,
) {
    assert_eq!(ConversationState::from_str(text), Ok(expected));
}

#[rstest]
#[case("pending", HookExecutionStatus::Pending)]
#[case("succeeded", HookExecutionStatus::Succeeded)]
#[case("failed", HookExecutionStatus::Failed)]
#[case("partial_failure", HookExecutionStatus::PartialFailure)]
fn hook_execution_status_parses_its_wire_names(
    #[case] text: &str,
    #[case] expected: HookExecutionStatus,
) {
    assert_eq!(HookExecutionStatus::from_str(text), Ok(expected));
}

#[rstest]
#[case("unknown", McpServerHealthStatus::Unknown)]
#[case("healthy", McpServerHealthStatus::Healthy)]
#[case("unhealthy", McpServerHealthStatus::Unhealthy)]
fn mcp_server_health_status_parses_its_wire_names(
    #[case] text: &str,
    #[case] expected: McpServerHealthStatus,
) {
    assert_eq!(McpServerHealthStatus::from_str(text), Ok(expected));
}

#[rstest]
#[case("registered", McpServerLifecycleState::Registered)]
#[case("running", McpServerLifecycleState::Running)]
#[case("stopped", McpServerLifecycleState::Stopped)]
fn mcp_server_lifecycle_state_parses_its_wire_names(
    #[case] text: &str,
    #[case] expected: McpServerLifecycleState,
) {
    assert_eq!(McpServerLifecycleState::from_str(text), Ok(expected));
}

#[rstest]
#[case("session_start", SnapshotType::SessionStart)]
#[case("handoff_initiated", SnapshotType::HandoffInitiated)]
#[case("truncation", SnapshotType::Truncation)]
#[case("checkpoint", SnapshotType::Checkpoint)]
fn snapshot_type_parses_its_wire_names(#[case] text: &str, #[case] expected: SnapshotType) {
    assert_eq!(SnapshotType::from_str(text), Ok(expected));
}

#[rstest]
#[case("active", TurnSessionStatus::Active)]
#[case("reserved", TurnSessionStatus::Reserved)]
#[case("expired", TurnSessionStatus::Expired)]
fn turn_session_status_parses_its_names(#[case] text: &str, #[case] expected: TurnSessionStatus) {
    assert_eq!(TurnSessionStatus::from_str(text), Ok(expected));
}

#[rstest]
#[case("")]
#[case("not-a-state")]
#[case("ACTIVE ")]
fn unknown_text_is_refused_by_every_parser(#[case] text: &str) {
    // Empty and unrelated text is refused by all seven parsers; the padded,
    // upper-case form is refused by the exact-match ones (the rest normalise).
    assert!(AgentSessionState::from_str(text).is_err());
    assert!(ConversationState::from_str(text).is_err());
    assert!(HookExecutionStatus::from_str(text).is_err());
    assert!(SnapshotType::from_str(text).is_err());
    if text != "ACTIVE " {
        assert!(McpServerHealthStatus::from_str(text).is_err());
        assert!(McpServerLifecycleState::from_str(text).is_err());
        assert!(TurnSessionStatus::from_str(text).is_err());
    }
}
