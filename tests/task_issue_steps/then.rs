//! Then steps for task lifecycle BDD scenarios.

use super::world::{TaskWorld, run_async};
use corbusier::task::{
    domain::{IssueRef, IssueSnapshot, Task, TaskOrigin, TaskState},
    ports::TaskRepositoryError,
    services::TaskLifecycleError,
};
use rstest_bdd_macros::then;

fn assert_lifecycle_data(task: &Task) -> Result<(), eyre::Report> {
    if task.state() != TaskState::Draft {
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
    check_eq(
        "issue provider",
        &expected_provider,
        &issue_ref.provider().as_str(),
    )?;
    check_eq(
        "issue repository",
        &expected_repository,
        &issue_ref.repository().as_str(),
    )?;
    check_eq(
        "issue number",
        &expected_issue_number,
        &issue_ref.issue_number().value(),
    )?;

    Ok(())
}

fn check_eq<T: PartialEq + std::fmt::Display>(
    label: &str,
    expected: &T,
    actual: &T,
) -> Result<(), eyre::Report> {
    if actual != expected {
        return Err(eyre::eyre!("expected {label} {expected}, found {actual}"));
    }

    Ok(())
}

fn assert_issue_title(metadata: &IssueSnapshot, expected_title: &str) -> Result<(), eyre::Report> {
    if metadata.title() != expected_title {
        return Err(eyre::eyre!(
            "expected issue title {}, found {}",
            expected_title,
            metadata.title()
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
    let lookup_result = run_async(world.service.find_by_issue_ref(&world.ctx, &issue_ref));
    let found = record_lookup(world, lookup_result)?;

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

fn record_lookup(
    world: &mut TaskWorld,
    result: Result<Option<Task>, TaskLifecycleError>,
) -> Result<Option<Task>, eyre::Report> {
    match result {
        Ok(found) => {
            world.last_lookup_result = Some(Ok(found.clone()));
            Ok(found)
        }
        Err(err) => {
            world.last_lookup_result = Some(Err(err));
            let display = world
                .last_lookup_result
                .as_ref()
                .and_then(|stored| stored.as_ref().err())
                .map(std::string::ToString::to_string)
                .ok_or_else(|| eyre::eyre!("lookup failed: missing stored error"))?;
            Err(eyre::eyre!("lookup failed: {display}"))
        }
    }
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

#[then("both tenants successfully create distinct tasks from the same issue")]
fn both_tenants_create_distinct_tasks(world: &TaskWorld) -> Result<(), eyre::Report> {
    let task_a = world
        .last_create_result
        .as_ref()
        .ok_or_else(|| eyre::eyre!("tenant A create result is missing"))?
        .as_ref()
        .map_err(|err| eyre::eyre!("tenant A task creation failed: {err}"))?;
    let task_b = world
        .other_create_result
        .as_ref()
        .ok_or_else(|| eyre::eyre!("tenant B create result is missing"))?
        .as_ref()
        .map_err(|err| eyre::eyre!("tenant B task creation failed: {err}"))?;

    if task_a.id() == task_b.id() {
        return Err(eyre::eyre!("tenant task IDs must be distinct"));
    }
    Ok(())
}

#[then("each tenant can retrieve its own task by the external issue reference")]
fn each_tenant_retrieves_own_task(world: &TaskWorld) -> Result<(), eyre::Report> {
    let issue_ref = world
        .pending_lookup
        .clone()
        .ok_or_else(|| eyre::eyre!("missing issue reference for retrieval step"))?;
    let task_a = world
        .last_created_task
        .as_ref()
        .ok_or_else(|| eyre::eyre!("tenant A created task is missing"))?;
    let task_b = world
        .other_created_task
        .as_ref()
        .ok_or_else(|| eyre::eyre!("tenant B created task is missing"))?;

    let found_a = run_async(world.service.find_by_issue_ref(&world.ctx, &issue_ref))
        .map_err(|err| eyre::eyre!("tenant A lookup failed: {err}"))?
        .ok_or_else(|| eyre::eyre!("tenant A task not found"))?;
    let found_b = run_async(
        world
            .service
            .find_by_issue_ref(&world.other_ctx, &issue_ref),
    )
    .map_err(|err| eyre::eyre!("tenant B lookup failed: {err}"))?
    .ok_or_else(|| eyre::eyre!("tenant B task not found"))?;

    assert_lookup_matches(&found_a, task_a, "tenant A")?;
    assert_lookup_matches(&found_b, task_b, "tenant B")?;
    Ok(())
}

fn assert_lookup_matches(found: &Task, expected: &Task, label: &str) -> Result<(), eyre::Report> {
    if found != expected {
        return Err(eyre::eyre!("{label} lookup returned the wrong task"));
    }

    Ok(())
}
