//! Given steps for hook engine execution scenarios.

use super::world::HookWorld;
use corbusier::hook_engine::domain::{
    ActionStatus, HookAction, HookActionId, HookActionType, HookDefinition, HookId, HookTriggerType,
};
use rstest_bdd_macros::given;

struct HookSetup {
    hook_id: &'static str,
    trigger: HookTriggerType,
    action_id: &'static str,
    action_type: HookActionType,
}

fn configure_hook(world: &mut HookWorld, setup: HookSetup) -> Result<HookActionId, eyre::Report> {
    let hook_id = HookId::new(setup.hook_id).map_err(|err| eyre::eyre!(err))?;
    let action_id = HookActionId::new(setup.action_id).map_err(|err| eyre::eyre!(err))?;
    let definition = HookDefinition::new(
        hook_id,
        format!("Hook {action_id}"),
        setup.trigger,
        vec![HookAction::new(action_id.clone(), setup.action_type)],
    )
    .map_err(|err| eyre::eyre!(err))?;
    world
        .definition_repo
        .insert(definition)
        .map_err(|err| eyre::eyre!(err))?;
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
        .map_err(|err| eyre::eyre!(err))?;
    Ok(())
}
