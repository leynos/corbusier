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

/// Result type for in-memory handoff tests.
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
    pub snapshot_adapter: Arc<InMemoryContextSnapshotAdapter>,
    pub service: HandoffService<
        InMemoryAgentSessionRepository,
        InMemoryHandoffAdapter<DefaultClock>,
        InMemoryContextSnapshotAdapter,
        DefaultClock,
    >,
}

impl HandoffTestHarness {
    /// Creates a test harness with in-memory adapters and a default clock.
    ///
    /// # Examples
    ///
    /// ```ignore
    /// let harness = HandoffTestHarness::new();
    /// assert!(harness.session_repo.is_empty());
    /// ```
    pub fn new() -> Self {
        let clock = Arc::new(DefaultClock);
        let session_repo = Arc::new(InMemoryAgentSessionRepository::new());
        let handoff_adapter = Arc::new(InMemoryHandoffAdapter::new(DefaultClock));
        let snapshot_adapter = Arc::new(InMemoryContextSnapshotAdapter::new());

        let service = HandoffService::new(
            Arc::clone(&session_repo),
            Arc::clone(&handoff_adapter),
            Arc::clone(&snapshot_adapter),
            clock,
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
