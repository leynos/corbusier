//! When steps for hook engine execution scenarios.

use super::world::{HookWorld, run_async};
use corbusier::hook_engine::domain::{HookTriggerContext, HookTriggerType};
use corbusier::hook_engine::ports::HookEngine;
use eyre::WrapErr;
use mockable::DefaultClock;
use rstest_bdd_macros::when;

fn fire_trigger(world: &mut HookWorld, trigger: HookTriggerType) -> Result<(), eyre::Report> {
    let context = HookTriggerContext::new(trigger, &DefaultClock);
    let stored_context = context.clone();
    let results = run_async(world.service.execute(context)).wrap_err("hook execution failed")?;
    world.last_context = Some(stored_context);
    world.last_results = Some(results);
    Ok(())
}

#[when("the pre-commit hook trigger fires")]
fn pre_commit_trigger_fires(world: &mut HookWorld) -> Result<(), eyre::Report> {
    fire_trigger(world, HookTriggerType::PreCommit)
}

#[when("the post-deploy hook trigger fires")]
fn post_deploy_trigger_fires(world: &mut HookWorld) -> Result<(), eyre::Report> {
    fire_trigger(world, HookTriggerType::PostDeploy)
}
