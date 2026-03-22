//! Then steps for hook-backed tool policy enforcement scenarios.

use super::world::{HookPolicyWorld, run_async};
use corbusier::hook_engine::ports::HookPolicyAuditRepository;
use corbusier::tool_registry::domain::ToolRegistryDomainError;
use corbusier::tool_registry::services::ToolDiscoveryRoutingServiceError;
use eyre::{WrapErr, eyre};
use rstest_bdd_macros::then;

#[then("the policy audit is retrievable by conversation")]
fn policy_audit_is_retrievable_by_conversation(
    world: &mut HookPolicyWorld,
) -> Result<(), eyre::Report> {
    let conversation_id = world
        .last_conversation_id
        .ok_or_else(|| eyre!("conversation id should be recorded"))?;
    let events = run_async(
        world
            .policy_audit
            .find_by_conversation(&world.request_ctx, conversation_id),
    )
    .wrap_err("query policy audit by conversation")?;
    world.last_events.clone_from(&events);
    if world.last_result.is_none() {
        return Err(eyre!("expected successful tool call result"));
    }
    if events.len() != 1 {
        return Err(eyre!("expected 1 policy audit event, got {}", events.len()));
    }
    Ok(())
}

#[then("the policy audit is retrievable by task")]
fn policy_audit_is_retrievable_by_task(world: &mut HookPolicyWorld) -> Result<(), eyre::Report> {
    let task_id = world
        .last_task_id
        .ok_or_else(|| eyre!("task id should be recorded"))?;
    let events = run_async(world.policy_audit.find_by_task(&world.request_ctx, task_id))
        .wrap_err("query policy audit by task")?;
    world.last_events.clone_from(&events);
    let Some(err) = &world.last_error else {
        return Err(eyre!("expected policy denial error"));
    };
    if !matches!(
        err,
        ToolDiscoveryRoutingServiceError::Domain(ToolRegistryDomainError::PolicyDenied { .. })
    ) {
        return Err(eyre!("expected policy denial error, got {err}"));
    }
    if events.len() != 1 {
        return Err(eyre!("expected 1 policy audit event, got {}", events.len()));
    }
    Ok(())
}

#[then("the policy audit is retrievable by hook event")]
fn policy_audit_is_retrievable_by_hook_event(
    world: &mut HookPolicyWorld,
) -> Result<(), eyre::Report> {
    let Some(result) = &world.last_result else {
        return Err(eyre!("expected successful tool call result"));
    };
    let conversation_id = world
        .last_conversation_id
        .ok_or_else(|| eyre!("conversation id should be recorded"))?;
    let events = run_async(
        world
            .policy_audit
            .find_by_conversation(&world.request_ctx, conversation_id),
    )
    .wrap_err("query policy audit by conversation")?;
    if events.len() != 1 {
        return Err(eyre!("expected 1 policy audit event, got {}", events.len()));
    }
    let trigger_context_id = events
        .first()
        .ok_or_else(|| eyre!("expected policy audit event"))?
        .trigger_context_id();
    let by_event = run_async(
        world
            .policy_audit
            .find_by_trigger_context(&world.request_ctx, trigger_context_id),
    )
    .wrap_err("query policy audit by hook event")?;
    world.last_events.clone_from(&by_event);
    if by_event.len() != 1 {
        return Err(eyre!(
            "expected 1 policy audit event by trigger, got {}",
            by_event.len()
        ));
    }
    if !result.outcome().is_success() {
        return Err(eyre!("expected successful tool call outcome"));
    }
    Ok(())
}
