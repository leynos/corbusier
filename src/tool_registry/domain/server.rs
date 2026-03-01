//! MCP server registration aggregate root.

use super::{
    McpServerHealthSnapshot, McpServerId, McpServerName, McpTransport,
    ParseMcpServerLifecycleStateError, ToolRegistryDomainError,
};
use chrono::{DateTime, Utc};
use mockable::Clock;
use serde::{Deserialize, Serialize};
use std::fmt;

/// Lifecycle state of a registered MCP server.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum McpServerLifecycleState {
    /// Server is registered but not currently running.
    Registered,
    /// Server is currently running.
    Running,
    /// Server was previously started and then stopped.
    Stopped,
}

impl McpServerLifecycleState {
    /// Returns the canonical storage representation.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Registered => "registered",
            Self::Running => "running",
            Self::Stopped => "stopped",
        }
    }

    /// Returns whether this state allows querying tools.
    #[must_use]
    pub const fn can_query_tools(self) -> bool {
        matches!(self, Self::Running)
    }

    /// Returns whether transition to `target` is allowed.
    #[must_use]
    pub const fn can_transition_to(self, target: Self) -> bool {
        matches!(
            (self, target),
            (
                Self::Registered,
                Self::Registered | Self::Running | Self::Stopped
            ) | (Self::Running | Self::Stopped, Self::Running | Self::Stopped)
        )
    }
}

impl fmt::Display for McpServerLifecycleState {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

impl TryFrom<&str> for McpServerLifecycleState {
    type Error = ParseMcpServerLifecycleStateError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        let normalized = value.trim().to_ascii_lowercase();
        match normalized.as_str() {
            "registered" => Ok(Self::Registered),
            "running" => Ok(Self::Running),
            "stopped" => Ok(Self::Stopped),
            _ => Err(ParseMcpServerLifecycleStateError(value.to_owned())),
        }
    }
}

/// MCP server registration aggregate root.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct McpServerRegistration {
    id: McpServerId,
    name: McpServerName,
    transport: McpTransport,
    lifecycle_state: McpServerLifecycleState,
    last_health: Option<McpServerHealthSnapshot>,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

/// Parameter object for reconstructing persisted server state.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PersistedMcpServerData {
    /// Persisted server identifier.
    pub id: McpServerId,
    /// Persisted server name.
    pub name: McpServerName,
    /// Persisted transport settings.
    pub transport: McpTransport,
    /// Persisted lifecycle state.
    pub lifecycle_state: McpServerLifecycleState,
    /// Persisted last health snapshot.
    pub last_health: Option<McpServerHealthSnapshot>,
    /// Persisted creation timestamp.
    pub created_at: DateTime<Utc>,
    /// Persisted update timestamp.
    pub updated_at: DateTime<Utc>,
}

impl McpServerRegistration {
    /// Creates a new MCP server registration.
    #[must_use]
    pub fn new(name: McpServerName, transport: McpTransport, clock: &impl Clock) -> Self {
        let timestamp = clock.utc();
        Self {
            id: McpServerId::new(),
            name,
            transport,
            lifecycle_state: McpServerLifecycleState::Registered,
            last_health: Some(McpServerHealthSnapshot::unknown(timestamp)),
            created_at: timestamp,
            updated_at: timestamp,
        }
    }

    /// Reconstructs a registration from persistence.
    #[must_use]
    pub fn from_persisted(data: PersistedMcpServerData) -> Self {
        Self {
            id: data.id,
            name: data.name,
            transport: data.transport,
            lifecycle_state: data.lifecycle_state,
            last_health: data.last_health,
            created_at: data.created_at,
            updated_at: data.updated_at,
        }
    }

    /// Returns the server identifier.
    #[must_use]
    pub const fn id(&self) -> McpServerId {
        self.id
    }

    /// Returns the validated server name.
    #[must_use]
    pub const fn name(&self) -> &McpServerName {
        &self.name
    }

    /// Returns the transport settings.
    #[must_use]
    pub const fn transport(&self) -> &McpTransport {
        &self.transport
    }

    /// Returns the lifecycle state.
    #[must_use]
    pub const fn lifecycle_state(&self) -> McpServerLifecycleState {
        self.lifecycle_state
    }

    /// Returns the latest health snapshot.
    #[must_use]
    pub const fn last_health(&self) -> Option<&McpServerHealthSnapshot> {
        self.last_health.as_ref()
    }

    /// Returns the creation timestamp.
    #[must_use]
    pub const fn created_at(&self) -> DateTime<Utc> {
        self.created_at
    }

    /// Returns the latest update timestamp.
    #[must_use]
    pub const fn updated_at(&self) -> DateTime<Utc> {
        self.updated_at
    }

    /// Marks the server as started and stores the latest health snapshot.
    ///
    /// # Errors
    ///
    /// Returns [`ToolRegistryDomainError::InvalidLifecycleTransition`] when the
    /// transition is not allowed.
    pub fn mark_started(
        &mut self,
        health_snapshot: McpServerHealthSnapshot,
        clock: &impl Clock,
    ) -> Result<(), ToolRegistryDomainError> {
        self.transition_to(McpServerLifecycleState::Running)?;
        self.last_health = Some(health_snapshot);
        self.touch(clock);
        Ok(())
    }

    /// Marks the server as stopped.
    ///
    /// # Errors
    ///
    /// Returns [`ToolRegistryDomainError::InvalidLifecycleTransition`] when the
    /// transition is not allowed.
    pub fn mark_stopped(&mut self, clock: &impl Clock) -> Result<(), ToolRegistryDomainError> {
        self.transition_to(McpServerLifecycleState::Stopped)?;
        self.last_health = Some(McpServerHealthSnapshot::unknown(clock.utc()));
        self.touch(clock);
        Ok(())
    }

    /// Updates the latest health snapshot in place.
    pub fn update_health(&mut self, health_snapshot: McpServerHealthSnapshot, clock: &impl Clock) {
        self.last_health = Some(health_snapshot);
        self.touch(clock);
    }

    /// Validates that querying tools is allowed.
    ///
    /// # Errors
    ///
    /// Returns [`ToolRegistryDomainError::ToolQueryRequiresRunning`] when the
    /// lifecycle state is not `running`.
    pub fn ensure_can_query_tools(&self) -> Result<(), ToolRegistryDomainError> {
        if self.lifecycle_state.can_query_tools() {
            return Ok(());
        }

        Err(ToolRegistryDomainError::ToolQueryRequiresRunning {
            server_id: self.id,
            state: self.lifecycle_state.as_str().to_owned(),
        })
    }

    fn touch(&mut self, clock: &impl Clock) {
        self.updated_at = clock.utc();
    }

    fn transition_to(
        &mut self,
        target_state: McpServerLifecycleState,
    ) -> Result<(), ToolRegistryDomainError> {
        if !self.lifecycle_state.can_transition_to(target_state) {
            return Err(ToolRegistryDomainError::InvalidLifecycleTransition {
                from: self.lifecycle_state.as_str().to_owned(),
                to: target_state.as_str().to_owned(),
            });
        }

        self.lifecycle_state = target_state;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tool_registry::domain::{McpServerHealthStatus, McpTransport};
    use chrono::Utc;
    use mockable::DefaultClock;
    use rstest::rstest;

    fn build_registration(clock: &DefaultClock) -> McpServerRegistration {
        let name = McpServerName::new("workspace_tools").expect("valid server name");
        let transport = McpTransport::stdio("mcp-server").expect("valid transport");
        McpServerRegistration::new(name, transport, clock)
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

        assert_eq!(
            registration.lifecycle_state(),
            McpServerLifecycleState::Running
        );
        assert_eq!(
            registration
                .last_health()
                .expect("health snapshot should exist")
                .status(),
            McpServerHealthStatus::Healthy
        );
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
}
