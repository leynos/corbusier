//! Then steps for task lifecycle BDD scenarios.

use super::world::{TaskWorld, run_async};
use corbusier::task::{
    domain::{IssueRef, IssueSnapshot, Task, TaskOrigin},
    ports::TaskRepositoryError,
    services::TaskLifecycleError,
};
use rstest_bdd_macros::then;

fn assert_lifecycle_data(task: &Task) -> Result<(), eyre::Report> {
    if task.state().as_str() != "draft" {
        return Err(eyre::eyre!(
            "expected draft state, found {}",
            task.state().as_str()
        ));
    }
    if task.created_at() != task.updated_at() {
        return Err(eyre::eyre!(
            "expected created_at and updated_at timestamps to match at creation"
        ));
    }
    Ok(())
}

fn assert_issue_reference(
    issue_ref: &IssueRef,
    expected_provider: &str,
    expected_repository: &str,
    expected_issue_number: u64,
) -> Result<(), eyre::Report> {
    if issue_ref.provider().as_str() != expected_provider {
        return Err(eyre::eyre!(
            "expected issue provider {}, found {}",
            expected_provider,
            issue_ref.provider().as_str()
        ));
    }
    if issue_ref.repository().as_str() != expected_repository {
        return Err(eyre::eyre!(
            "expected issue repository {}, found {}",
            expected_repository,
            issue_ref.repository().as_str()
        ));
    }
    if issue_ref.issue_number().value() != expected_issue_number {
        return Err(eyre::eyre!(
            "expected issue number {}, found {}",
            expected_issue_number,
            issue_ref.issue_number().value()
        ));
    }

    Ok(())
}

fn assert_issue_title(metadata: &IssueSnapshot, expected_title: &str) -> Result<(), eyre::Report> {
    if metadata.title != expected_title {
        return Err(eyre::eyre!(
            "expected issue title {}, found {}",
            expected_title,
            metadata.title
        ));
    }

    Ok(())
}

#[then("the task is created with draft state and lifecycle timestamps")]
fn task_created_with_lifecycle_data(world: &TaskWorld) -> Result<(), eyre::Report> {
    let create_result = world
        .last_create_result
        .as_ref()
        .ok_or_else(|| eyre::eyre!("missing create result in scenario world"))?;
    let task = create_result
        .as_ref()
        .map_err(|err| eyre::eyre!("unexpected task creation failure: {err}"))?;

    assert_lifecycle_data(task)?;

    let (expected_provider, expected_repository, expected_issue_number) = world
        .pending_issue_ref
        .clone()
        .ok_or_else(|| eyre::eyre!("missing expected issue reference in scenario world"))?;
    let expected_title = world
        .pending_issue_title
        .clone()
        .ok_or_else(|| eyre::eyre!("missing expected issue title in scenario world"))?;
    let TaskOrigin::Issue {
        issue_ref,
        metadata,
        ..
    } = task.origin();
    assert_issue_reference(
        issue_ref,
        &expected_provider,
        &expected_repository,
        expected_issue_number,
    )?;
    assert_issue_title(metadata, &expected_title)?;

    Ok(())
}

#[then("the task can be retrieved by the external issue reference")]
fn task_retrievable_by_issue_reference(world: &mut TaskWorld) -> Result<(), eyre::Report> {
    let issue_ref = world
        .pending_lookup
        .clone()
        .ok_or_else(|| eyre::eyre!("missing issue reference for retrieval step"))?;
    let found = run_async(world.service.find_by_issue_ref(&issue_ref))
        .map_err(|err| eyre::eyre!("lookup failed: {err}"))?;

    if found.is_none() {
        return Err(eyre::eyre!(
            "expected task lookup by issue reference to return a task"
        ));
    }
    if let (Some(created), Some(fetched)) = (world.last_created_task.as_ref(), found.as_ref())
        && created != fetched
    {
        return Err(eyre::eyre!("lookup task does not match created task"));
    }
    Ok(())
}

#[then("task creation fails with a duplicate issue reference error")]
fn duplicate_issue_creation_error(world: &TaskWorld) -> Result<(), eyre::Report> {
    let result = world
        .last_create_result
        .as_ref()
        .ok_or_else(|| eyre::eyre!("missing create result in scenario world"))?;

    if !matches!(
        result,
        Err(TaskLifecycleError::Repository(
            TaskRepositoryError::DuplicateIssueOrigin(_)
        ))
    ) {
        return Err(eyre::eyre!(
            "expected duplicate issue reference error, got {result:?}"
        ));
    }
    Ok(())
}

#[then("no task is returned")]
fn no_task_is_returned(world: &TaskWorld) -> Result<(), eyre::Report> {
    let lookup_result = world
        .last_lookup_result
        .as_ref()
        .ok_or_else(|| eyre::eyre!("missing lookup result in scenario world"))?;
    let found = lookup_result
        .as_ref()
        .map_err(|err| eyre::eyre!("unexpected lookup error: {err}"))?;
    if found.is_some() {
        return Err(eyre::eyre!(
            "expected no task for unknown issue reference lookup"
        ));
    }
    Ok(())
}
