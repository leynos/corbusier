//! Given steps for task lifecycle BDD scenarios.

use super::world::{TaskWorld, run_async};
use corbusier::task::{domain::IssueRef, services::CreateTaskFromIssueRequest};
use eyre::WrapErr;
use rstest_bdd_macros::given;

#[given(r#"an external issue "{provider}" "{repository}" #{issue_number:u64}"#)]
fn external_issue(world: &mut TaskWorld, provider: String, repository: String, issue_number: u64) {
    world.pending_issue_ref = Some((provider, repository, issue_number));
}

#[given(r#"the issue has title "{title}""#)]
fn issue_has_title(world: &mut TaskWorld, title: String) -> Result<(), eyre::Report> {
    let (provider, repository, issue_number) = world
        .pending_issue_ref
        .clone()
        .ok_or_else(|| eyre::eyre!("missing pending issue reference in scenario world"))?;
    world.pending_request = Some(CreateTaskFromIssueRequest::new(
        provider,
        repository,
        issue_number,
        title,
    ));
    Ok(())
}

#[given("a task has already been created from that issue")]
fn task_already_exists(world: &mut TaskWorld) -> Result<(), eyre::Report> {
    let request = world
        .pending_request
        .clone()
        .ok_or_else(|| eyre::eyre!("missing pending request in scenario world"))?;
    let created = run_async(world.service.create_from_issue(request))
        .wrap_err("create initial task for duplicate scenario")?;

    world.last_created_task = Some(created);
    Ok(())
}

#[given(r#"an unknown issue reference "{provider}" "{repository}" #{issue_number:u64}"#)]
fn unknown_issue_reference(
    world: &mut TaskWorld,
    provider: String,
    repository: String,
    issue_number: u64,
) -> Result<(), eyre::Report> {
    world.pending_lookup = Some(
        IssueRef::from_parts(&provider, &repository, issue_number)
            .wrap_err("construct unknown issue reference")?,
    );
    Ok(())
}
