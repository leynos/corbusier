//! Given steps for hook engine execution scenarios.

use super::world::{HookWorld, run_async};
use corbusier::hook_engine::domain::{
    ActionStatus, HookAction, HookActionId, HookActionType, HookDefinition, HookId, HookTriggerType,
};
use eyre::WrapErr;
use rstest_bdd_macros::given;
use serde_json::json;

struct HookSetup {
    hook_id: &'static str,
    trigger: HookTriggerType,
    action_id: &'static str,
    action_type: HookActionType,
}

fn configure_hook(world: &mut HookWorld, setup: HookSetup) -> Result<HookActionId, eyre::Report> {
    let hook_id =
        HookId::new(setup.hook_id).wrap_err("build hook identifier for scenario setup")?;
    let action_id = HookActionId::new(setup.action_id)
        .wrap_err("build hook action identifier for scenario setup")?;
    let definition = HookDefinition::new(
        hook_id,
        format!("Hook {action_id}"),
        setup.trigger,
        vec![HookAction::new(action_id.clone(), setup.action_type)],
    )
    .wrap_err("build hook definition for scenario setup")?;
    run_async(world.definition_repo.insert(&world.request_ctx, definition))
        .wrap_err("insert hook definition into in-memory scenario repository")?;
    Ok(action_id)
}

#[given("a pre-commit hook is configured")]
fn pre_commit_hook_configured(world: &mut HookWorld) -> Result<(), eyre::Report> {
    configure_hook(
        world,
        HookSetup {
            hook_id: "hook-pre-commit",
            trigger: HookTriggerType::PreCommit,
            action_id: "action-pre-commit",
            action_type: HookActionType::QualityGate,
        },
    )?;
    Ok(())
}

#[given("a post-deploy hook is configured to fail")]
fn post_deploy_hook_configured_to_fail(world: &mut HookWorld) -> Result<(), eyre::Report> {
    let action_id = configure_hook(
        world,
        HookSetup {
            hook_id: "hook-post-deploy",
            trigger: HookTriggerType::PostDeploy,
            action_id: "action-post-deploy",
            action_type: HookActionType::PolicyCheck,
        },
    )?;
    world
        .action_executor
        .set_outcome(action_id.as_str(), ActionStatus::Failed)
        .wrap_err("configure failing action outcome for scenario hook")?;
    world
        .action_executor
        .set_output(
            action_id.as_str(),
            json!({
                "decision": "deny",
                "reason": "post-deploy validation failed",
            }),
        )
        .wrap_err("configure failing policy audit output for scenario hook")?;
    Ok(())
}
