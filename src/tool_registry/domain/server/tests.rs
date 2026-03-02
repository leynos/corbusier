//! Unit tests for MCP server registration lifecycle behaviour.

use crate::tool_registry::domain::{
    McpServerHealthSnapshot, McpServerHealthStatus, McpServerLifecycleState, McpServerName,
    McpServerRegistration, McpTransport, ToolRegistryDomainError,
};
use chrono::Utc;
use mockable::DefaultClock;
use rstest::rstest;

fn build_registration(clock: &DefaultClock) -> McpServerRegistration {
    let name = McpServerName::new("workspace_tools").expect("valid server name");
    let transport = McpTransport::stdio("mcp-server").expect("valid transport");
    McpServerRegistration::new(name, transport, clock)
}

/// Helper to assert lifecycle state and health status in one call.
fn assert_state_and_health(
    registration: &McpServerRegistration,
    expected_state: McpServerLifecycleState,
    expected_health_status: McpServerHealthStatus,
) {
    assert_eq!(registration.lifecycle_state(), expected_state);
    assert_eq!(
        registration
            .last_health()
            .expect("health snapshot should exist")
            .status(),
        expected_health_status
    );
}

#[test]
fn registration_starts_in_registered_state() {
    let clock = DefaultClock;
    let registration = build_registration(&clock);

    assert_eq!(
        registration.lifecycle_state(),
        McpServerLifecycleState::Registered
    );
    let health_snapshot = registration
        .last_health()
        .expect("health snapshot should exist");
    assert_eq!(health_snapshot.status(), McpServerHealthStatus::Unknown);
}

#[rstest]
#[case(
    McpServerLifecycleState::Registered,
    McpServerLifecycleState::Running,
    true
)]
#[case(
    McpServerLifecycleState::Registered,
    McpServerLifecycleState::Stopped,
    true
)]
#[case(
    McpServerLifecycleState::Running,
    McpServerLifecycleState::Stopped,
    true
)]
#[case(
    McpServerLifecycleState::Stopped,
    McpServerLifecycleState::Running,
    true
)]
#[case(
    McpServerLifecycleState::Running,
    McpServerLifecycleState::Registered,
    false
)]
#[case(
    McpServerLifecycleState::Stopped,
    McpServerLifecycleState::Registered,
    false
)]
fn lifecycle_transition_matrix(
    #[case] current: McpServerLifecycleState,
    #[case] target: McpServerLifecycleState,
    #[case] expected: bool,
) {
    assert_eq!(current.can_transition_to(target), expected);
}

#[test]
fn mark_started_updates_state_and_health() {
    let clock = DefaultClock;
    let mut registration = build_registration(&clock);
    let health = McpServerHealthSnapshot::healthy(Utc::now());
    registration
        .mark_started(health, &clock)
        .expect("start transition should succeed");

    assert_state_and_health(
        &registration,
        McpServerLifecycleState::Running,
        McpServerHealthStatus::Healthy,
    );
}

#[test]
fn mark_stopped_updates_state_and_resets_health() {
    let clock = DefaultClock;
    let mut registration = build_registration(&clock);
    let health = McpServerHealthSnapshot::healthy(Utc::now());
    registration
        .mark_started(health, &clock)
        .expect("start transition should succeed");

    registration
        .mark_stopped(&clock)
        .expect("stop transition should succeed");

    assert_state_and_health(
        &registration,
        McpServerLifecycleState::Stopped,
        McpServerHealthStatus::Unknown,
    );
}

#[test]
fn update_health_modifies_health_without_changing_state() {
    let clock = DefaultClock;
    let mut registration = build_registration(&clock);
    registration
        .mark_started(McpServerHealthSnapshot::healthy(Utc::now()), &clock)
        .expect("start transition should succeed");
    let lifecycle_state = registration.lifecycle_state();

    registration.update_health(
        McpServerHealthSnapshot::unhealthy(Utc::now(), "probe timeout"),
        &clock,
    );

    assert_eq!(registration.lifecycle_state(), lifecycle_state);
    let health = registration
        .last_health()
        .expect("health snapshot should exist");
    assert_eq!(health.status(), McpServerHealthStatus::Unhealthy);
    assert_eq!(health.message(), Some("probe timeout"));
}

#[test]
fn invalid_transition_attempts_return_error() {
    let clock = DefaultClock;
    let mut registration = build_registration(&clock);
    registration
        .mark_started(McpServerHealthSnapshot::healthy(Utc::now()), &clock)
        .expect("start transition should succeed");

    let result = registration.transition_to(McpServerLifecycleState::Registered);

    assert!(matches!(
        result,
        Err(ToolRegistryDomainError::InvalidLifecycleTransition { from, to })
        if from == "running" && to == "registered"
    ));
}

#[test]
fn ensure_can_query_tools_requires_running_state() {
    let clock = DefaultClock;
    let registration = build_registration(&clock);

    let result = registration.ensure_can_query_tools();
    assert!(matches!(
        result,
        Err(ToolRegistryDomainError::ToolQueryRequiresRunning { .. })
    ));
}
