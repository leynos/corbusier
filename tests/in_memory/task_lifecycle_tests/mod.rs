//! In-memory integration tests for task lifecycle operations.
//!
//! Tests are split by concern:
//! - `task_crud_tests`: Create, lookup, and duplicate detection
//! - `task_association_tests`: Branch and PR association
//! - `task_transition_tests`: State machine transitions
//! - `task_isolation_tests`: Cross-tenant isolation

mod task_association_tests;
mod task_crud_tests;
mod task_isolation_tests;
mod task_transition_tests;

use std::sync::Arc;

use corbusier::task::{adapters::memory::InMemoryTaskRepository, services::TaskLifecycleService};
use mockable::DefaultClock;
use rstest::fixture;

type TestService = TaskLifecycleService<InMemoryTaskRepository, DefaultClock>;

#[fixture]
fn service() -> TestService {
    TaskLifecycleService::new(
        Arc::new(InMemoryTaskRepository::new()),
        Arc::new(DefaultClock),
    )
}

/// Asserts exactly one task is found with the expected ID.
///
/// # Errors
///
/// Returns an error if the result set does not contain exactly one task
/// matching `expected_id`.
fn assert_single_task_found(
    found: &[corbusier::task::domain::Task],
    expected_id: corbusier::task::domain::TaskId,
) -> Result<(), eyre::Report> {
    eyre::ensure!(
        found.len() == 1,
        "expected exactly one task, found {}",
        found.len()
    );
    let task = found
        .first()
        .ok_or_else(|| eyre::eyre!("expected at least one task"))?;
    eyre::ensure!(task.id() == expected_id, "task ID mismatch");
    Ok(())
}
