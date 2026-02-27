//! Agent backend registration aggregate root.

use super::{AgentCapabilities, BackendId, BackendInfo, BackendName, BackendStatus};
use chrono::{DateTime, Utc};
use mockable::Clock;
use serde::{Deserialize, Serialize};

/// Agent backend registration aggregate root.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AgentBackendRegistration {
    id: BackendId,
    name: BackendName,
    status: BackendStatus,
    capabilities: AgentCapabilities,
    backend_info: BackendInfo,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

/// Parameter object for reconstructing a persisted backend registration.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PersistedBackendData {
    /// Persisted backend identifier.
    pub id: BackendId,
    /// Persisted backend name.
    pub name: BackendName,
    /// Persisted lifecycle status.
    pub status: BackendStatus,
    /// Persisted capability metadata.
    pub capabilities: AgentCapabilities,
    /// Persisted provider information.
    pub backend_info: BackendInfo,
    /// Persisted creation timestamp.
    pub created_at: DateTime<Utc>,
    /// Persisted latest lifecycle timestamp.
    pub updated_at: DateTime<Utc>,
}

impl AgentBackendRegistration {
    /// Creates a new backend registration with `Active` status.
    #[must_use]
    pub fn new(
        name: BackendName,
        capabilities: AgentCapabilities,
        backend_info: BackendInfo,
        clock: &impl Clock,
    ) -> Self {
        let timestamp = clock.utc();
        Self {
            id: BackendId::new(),
            name,
            status: BackendStatus::Active,
            capabilities,
            backend_info,
            created_at: timestamp,
            updated_at: timestamp,
        }
    }

    /// Reconstructs a registration from persisted storage.
    #[must_use]
    pub fn from_persisted(data: PersistedBackendData) -> Self {
        Self {
            id: data.id,
            name: data.name,
            status: data.status,
            capabilities: data.capabilities,
            backend_info: data.backend_info,
            created_at: data.created_at,
            updated_at: data.updated_at,
        }
    }

    /// Returns the backend identifier.
    #[must_use]
    pub const fn id(&self) -> BackendId {
        self.id
    }

    /// Returns the backend name.
    #[must_use]
    pub const fn name(&self) -> &BackendName {
        &self.name
    }

    /// Returns the backend lifecycle status.
    #[must_use]
    pub const fn status(&self) -> BackendStatus {
        self.status
    }

    /// Returns the capability metadata.
    #[must_use]
    pub const fn capabilities(&self) -> &AgentCapabilities {
        &self.capabilities
    }

    /// Returns the provider information.
    #[must_use]
    pub const fn backend_info(&self) -> &BackendInfo {
        &self.backend_info
    }

    /// Returns the creation timestamp.
    #[must_use]
    pub const fn created_at(&self) -> DateTime<Utc> {
        self.created_at
    }

    /// Returns the latest lifecycle timestamp.
    #[must_use]
    pub const fn updated_at(&self) -> DateTime<Utc> {
        self.updated_at
    }

    /// Deactivates the backend, setting status to [`BackendStatus::Inactive`].
    pub fn deactivate(&mut self, clock: &impl Clock) {
        self.status = BackendStatus::Inactive;
        self.touch(clock);
    }

    /// Activates the backend, setting status to [`BackendStatus::Active`].
    pub fn activate(&mut self, clock: &impl Clock) {
        self.status = BackendStatus::Active;
        self.touch(clock);
    }

    /// Replaces the capability metadata.
    pub fn update_capabilities(&mut self, capabilities: AgentCapabilities, clock: &impl Clock) {
        self.capabilities = capabilities;
        self.touch(clock);
    }

    /// Updates the `updated_at` timestamp to the current clock time.
    fn touch(&mut self, clock: &impl Clock) {
        self.updated_at = clock.utc();
    }
}
