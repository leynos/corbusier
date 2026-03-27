//! Then steps for hook engine execution scenarios.

use super::world::{HookWorld, run_async};
use corbusier::hook_engine::domain::HookExecutionStatus;
use corbusier::hook_engine::ports::{HookExecutionLogRepository, HookPolicyAuditRepository};
use eyre::WrapErr;
use rstest_bdd_macros::then;

fn assert_execution_status(
    world: &mut HookWorld,
    expected: HookExecutionStatus,
) -> Result<(), eyre::Report> {
    let results = world
        .last_results
        .as_ref()
        .ok_or_else(|| eyre::eyre!("no execution results captured"))?;
    let result = results
        .first()
        .ok_or_else(|| eyre::eyre!("no hook result"))?;
    if result.status() != expected {
        return Err(eyre::eyre!(
            "expected status {expected}, got {}",
            result.status()
        ));
    }

    let context = world
        .last_context
        .as_ref()
        .ok_or_else(|| eyre::eyre!("no trigger context captured"))?;
    let stored = run_async(
        world
            .execution_log
            .find_by_trigger_context(&world.request_ctx, context.id()),
    )
    .wrap_err("execution log lookup failed")?;
    if stored.len() != 1 {
        return Err(eyre::eyre!(
            "expected 1 stored execution, got {}",
            stored.len()
        ));
    }
    let stored_result = stored
        .first()
        .ok_or_else(|| eyre::eyre!("expected one stored execution result"))?;
    if stored_result.status() != expected {
        return Err(eyre::eyre!(
            "expected stored status {expected}, got {}",
            stored_result.status()
        ));
    }

    if expected == HookExecutionStatus::Failed {
        let audit_events = run_async(
            world
                .policy_audit
                .find_by_trigger_context(&world.request_ctx, context.id()),
        )
        .wrap_err("policy audit lookup failed")?;
        if audit_events.len() != 1 {
            return Err(eyre::eyre!(
                "expected 1 policy audit event for failed execution, got {}",
                audit_events.len()
            ));
        }
    }
    Ok(())
}

#[then("the hook execution is recorded as success")]
fn hook_execution_recorded_success(world: &mut HookWorld) -> Result<(), eyre::Report> {
    assert_execution_status(world, HookExecutionStatus::Succeeded)
}

#[then("the hook execution is recorded as failure")]
fn hook_execution_recorded_failure(world: &mut HookWorld) -> Result<(), eyre::Report> {
    assert_execution_status(world, HookExecutionStatus::Failed)
}
