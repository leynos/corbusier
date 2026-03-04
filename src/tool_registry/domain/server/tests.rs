//! Unit tests for MCP server registration lifecycle behaviour.

use crate::tool_registry::domain::{
    McpServerHealthSnapshot, McpServerHealthStatus, McpServerLifecycleState, McpServerName,
    McpServerRegistration, McpTransport, ToolRegistryDomainError,
};
use chrono::Utc;
use mockable::DefaultClock;
use rstest::{fixture, rstest};

#[fixture]
fn clock() -> DefaultClock {
    DefaultClock
}

#[fixture]
fn registration(clock: DefaultClock) -> McpServerRegistration {
    let name = McpServerName::new("workspace_tools").expect("valid server name");
    let transport = McpTransport::stdio("mcp-server").expect("valid transport");
    McpServerRegistration::new(name, transport, &clock)
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

#[rstest]
fn registration_starts_in_registered_state(registration: McpServerRegistration) {
    assert_state_and_health(
        &registration,
        McpServerLifecycleState::Registered,
        McpServerHealthStatus::Unknown,
    );
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
    false
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

#[rstest]
fn mark_started_updates_state_and_health(
    mut registration: McpServerRegistration,
    clock: DefaultClock,
) {
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

#[rstest]
fn mark_stopped_updates_state_and_resets_health(
    mut registration: McpServerRegistration,
    clock: DefaultClock,
) {
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

#[rstest]
fn update_health_modifies_health_without_changing_state(
    mut registration: McpServerRegistration,
    clock: DefaultClock,
) {
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

#[rstest]
fn invalid_transition_attempts_return_error(
    mut registration: McpServerRegistration,
    clock: DefaultClock,
) {
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

#[rstest]
fn ensure_can_query_tools_requires_running_state(registration: McpServerRegistration) {
    let result = registration.ensure_can_query_tools();
    assert!(matches!(
        result,
        Err(ToolRegistryDomainError::ToolQueryRequiresRunning { .. })
    ));
}
