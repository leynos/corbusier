//! When steps for task lifecycle BDD scenarios.

use super::world::{TaskWorld, run_async};
use corbusier::context::{CorrelationId, RequestContext, SessionId, TenantId, UserId};
use rstest_bdd_macros::when;

#[when("the issue is converted into a task")]
fn convert_issue_to_task(world: &mut TaskWorld) -> Result<(), eyre::Report> {
    let ctx = RequestContext::new(
        TenantId::new(),
        CorrelationId::new(),
        UserId::new(),
        SessionId::new(),
    );
    let request = world
        .pending_request
        .clone()
        .ok_or_else(|| eyre::eyre!("missing pending request in scenario world"))?;

    let result = run_async(world.service.create_from_issue(&ctx, request));
    if let Ok(task) = &result {
        world.last_created_task = Some(task.clone());
        world.pending_lookup = Some(task.origin().issue_ref().clone());
    }
    world.last_create_result = Some(result);
    Ok(())
}

#[when("the task is requested by external issue reference")]
fn lookup_by_issue_reference(world: &mut TaskWorld) -> Result<(), eyre::Report> {
    let ctx = RequestContext::new(
        TenantId::new(),
        CorrelationId::new(),
        UserId::new(),
        SessionId::new(),
    );
    let issue_ref = world
        .pending_lookup
        .clone()
        .ok_or_else(|| eyre::eyre!("missing pending lookup reference in scenario world"))?;
    world.last_lookup_result = Some(run_async(world.service.find_by_issue_ref(&ctx, &issue_ref)));
    Ok(())
}
