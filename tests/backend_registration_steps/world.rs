//! Shared world state for backend registration BDD scenarios.

use std::sync::Arc;

use corbusier::agent_backend::{
    adapters::memory::InMemoryBackendRegistry,
    domain::AgentBackendRegistration,
    services::{BackendRegistryService, BackendRegistryServiceError, RegisterBackendRequest},
};
use mockable::DefaultClock;
use rstest::fixture;

/// Service type used by the BDD world.
pub type TestRegistryService = BackendRegistryService<InMemoryBackendRegistry, DefaultClock>;

/// Pending backend specification before registration.
pub struct PendingBackend {
    /// Backend name.
    pub name: String,
    /// Provider name.
    pub provider: String,
}

/// Scenario world for backend registration behaviour tests.
pub struct BackendWorld {
    /// The registry service under test.
    pub service: TestRegistryService,
    /// Backends queued for registration.
    pub pending_backends: Vec<PendingBackend>,
    /// Last successfully registered backend.
    pub last_registered: Option<AgentBackendRegistration>,
    /// All registered backends (for multi-register scenarios).
    pub registered_backends: Vec<AgentBackendRegistration>,
    /// Result of the last registration attempt.
    pub last_register_result: Option<Result<AgentBackendRegistration, BackendRegistryServiceError>>,
    /// Result of the last `list_all` call.
    pub last_list_all_result: Option<Vec<AgentBackendRegistration>>,
    /// Result of the last `list_active` call.
    pub last_list_active_result: Option<Vec<AgentBackendRegistration>>,
}

impl BackendWorld {
    /// Creates a world with empty pending scenario state.
    #[must_use]
    pub fn new() -> Self {
        let service = BackendRegistryService::new(
            Arc::new(InMemoryBackendRegistry::new()),
            Arc::new(DefaultClock),
        );
        Self {
            service,
            pending_backends: Vec::new(),
            last_registered: None,
            registered_backends: Vec::new(),
            last_register_result: None,
            last_list_all_result: None,
            last_list_active_result: None,
        }
    }
}

impl Default for BackendWorld {
    fn default() -> Self {
        Self::new()
    }
}

/// Fixture that creates a new scenario world.
#[fixture]
pub fn world() -> BackendWorld {
    BackendWorld::default()
}

/// Runs an async operation within sync step definitions.
pub fn run_async<T>(future: impl std::future::Future<Output = T>) -> T {
    tokio::task::block_in_place(|| tokio::runtime::Handle::current().block_on(future))
}

/// Builds a [`RegisterBackendRequest`] from a name and provider.
pub fn build_request(name: &str, provider: &str) -> RegisterBackendRequest {
    RegisterBackendRequest::new(name, name, "1.0.0", provider).with_capabilities(true, true)
}
