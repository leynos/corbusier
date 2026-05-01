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
fn registration(clock: DefaultClock) -> Result<McpServerRegistration, eyre::Report> {
    let name = McpServerName::new("workspace_tools")?;
    let transport = McpTransport::stdio("mcp-server")?;
    Ok(McpServerRegistration::new(name, transport, &clock))
}

/// Helper to assert lifecycle state and health status in one call.
fn assert_state_and_health(
    registration: &McpServerRegistration,
    expected_state: McpServerLifecycleState,
    expected_health_status: McpServerHealthStatus,
) -> Result<(), eyre::Report> {
    eyre::ensure!(registration.lifecycle_state() == expected_state);
    let health = registration
        .last_health()
        .ok_or_else(|| eyre::eyre!("health snapshot should exist"))?;
    eyre::ensure!(health.status() == expected_health_status);
    Ok(())
}

#[rstest]
fn registration_starts_in_registered_state(
    registration: Result<McpServerRegistration, eyre::Report>,
) -> Result<(), eyre::Report> {
    let server_registration = registration?;
    assert_state_and_health(
        &server_registration,
        McpServerLifecycleState::Registered,
        McpServerHealthStatus::Unknown,
    )
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
    registration: Result<McpServerRegistration, eyre::Report>,
    clock: DefaultClock,
) -> Result<(), eyre::Report> {
    let mut server_registration = registration?;
    let health = McpServerHealthSnapshot::healthy(Utc::now());
    server_registration.mark_started(health, &clock)?;

    assert_state_and_health(
        &server_registration,
        McpServerLifecycleState::Running,
        McpServerHealthStatus::Healthy,
    )
}

#[rstest]
fn mark_stopped_updates_state_and_resets_health(
    registration: Result<McpServerRegistration, eyre::Report>,
    clock: DefaultClock,
) -> Result<(), eyre::Report> {
    let mut server_registration = registration?;
    let health = McpServerHealthSnapshot::healthy(Utc::now());
    server_registration.mark_started(health, &clock)?;

    server_registration.mark_stopped(&clock)?;

    assert_state_and_health(
        &server_registration,
        McpServerLifecycleState::Stopped,
        McpServerHealthStatus::Unknown,
    )
}

#[rstest]
fn update_health_modifies_health_without_changing_state(
    registration: Result<McpServerRegistration, eyre::Report>,
    clock: DefaultClock,
) -> Result<(), eyre::Report> {
    let mut server_registration = registration?;
    server_registration.mark_started(McpServerHealthSnapshot::healthy(Utc::now()), &clock)?;
    let lifecycle_state = server_registration.lifecycle_state();

    server_registration.update_health(
        McpServerHealthSnapshot::unhealthy(Utc::now(), "probe timeout"),
        &clock,
    );

    eyre::ensure!(server_registration.lifecycle_state() == lifecycle_state);
    let health = server_registration
        .last_health()
        .ok_or_else(|| eyre::eyre!("health snapshot should exist"))?;
    eyre::ensure!(health.status() == McpServerHealthStatus::Unhealthy);
    eyre::ensure!(health.message() == Some("probe timeout"));
    Ok(())
}

#[rstest]
fn invalid_transition_attempts_return_error(
    registration: Result<McpServerRegistration, eyre::Report>,
    clock: DefaultClock,
) -> Result<(), eyre::Report> {
    let mut server_registration = registration?;
    server_registration.mark_started(McpServerHealthSnapshot::healthy(Utc::now()), &clock)?;

    let result = server_registration.transition_to(McpServerLifecycleState::Registered);

    eyre::ensure!(matches!(
        result,
        Err(ToolRegistryDomainError::InvalidLifecycleTransition { ref from, ref to })
        if from == "running" && to == "registered"
    ));
    Ok(())
}

#[rstest]
fn ensure_can_query_tools_requires_running_state(
    registration: Result<McpServerRegistration, eyre::Report>,
) -> Result<(), eyre::Report> {
    let server_registration = registration?;
    let result = server_registration.ensure_can_query_tools();
    eyre::ensure!(matches!(
        result,
        Err(ToolRegistryDomainError::ToolQueryRequiresRunning { .. })
    ));
    Ok(())
}
