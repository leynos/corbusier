//! Shared harness for in-memory handoff integration tests.

use corbusier::message::{
    adapters::memory::{
        InMemoryAgentSessionRepository, InMemoryContextSnapshotAdapter, InMemoryHandoffAdapter,
    },
    services::HandoffService,
};
use mockable::DefaultClock;
use rstest::fixture;
use std::sync::Arc;
use tokio::runtime::Runtime;

pub type TestResult<T = ()> = Result<T, Box<dyn std::error::Error + Send + Sync>>;

/// Provides a tokio runtime for async operations in tests.
#[fixture]
pub fn runtime() -> TestResult<Runtime> {
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?;
    Ok(runtime)
}

/// Provides a clock for time-dependent operations.
#[fixture]
pub fn clock() -> DefaultClock {
    DefaultClock
}

/// A test harness containing all components needed for handoff testing.
pub struct HandoffTestHarness {
    pub session_repo: Arc<InMemoryAgentSessionRepository>,
    pub handoff_adapter: Arc<InMemoryHandoffAdapter<DefaultClock>>,
    pub snapshot_adapter: Arc<InMemoryContextSnapshotAdapter<DefaultClock>>,
    pub service: HandoffService<
        InMemoryAgentSessionRepository,
        InMemoryHandoffAdapter<DefaultClock>,
        InMemoryContextSnapshotAdapter<DefaultClock>,
    >,
}

impl HandoffTestHarness {
    pub fn new() -> Self {
        let session_repo = Arc::new(InMemoryAgentSessionRepository::new());
        let handoff_adapter = Arc::new(InMemoryHandoffAdapter::new(DefaultClock));
        let snapshot_adapter = Arc::new(InMemoryContextSnapshotAdapter::new(DefaultClock));

        let service = HandoffService::new(
            Arc::clone(&session_repo),
            Arc::clone(&handoff_adapter),
            Arc::clone(&snapshot_adapter),
        );

        Self {
            session_repo,
            handoff_adapter,
            snapshot_adapter,
            service,
        }
    }
}

#[fixture]
pub fn harness() -> HandoffTestHarness {
    HandoffTestHarness::new()
}
