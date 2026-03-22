//! Behaviour tests for hook-backed tool policy enforcement.

mod hook_policy_enforcement_steps;

use hook_policy_enforcement_steps::world::{HookPolicyWorld, world};
use rstest_bdd_macros::scenario;

#[scenario(
    path = "tests/features/hook_policy_enforcement.feature",
    name = "A pre-tool-use policy permits a tool call and is queryable by conversation"
)]
#[tokio::test(flavor = "multi_thread")]
async fn pre_tool_use_policy_permits(world: HookPolicyWorld) {
    let _ = world;
}

#[scenario(
    path = "tests/features/hook_policy_enforcement.feature",
    name = "A pre-tool-use policy denies a tool call and is queryable by task"
)]
#[tokio::test(flavor = "multi_thread")]
async fn pre_tool_use_policy_denies(world: HookPolicyWorld) {
    let _ = world;
}

#[scenario(
    path = "tests/features/hook_policy_enforcement.feature",
    name = "A post-tool-use hook records an audit event retrievable by hook event"
)]
#[tokio::test(flavor = "multi_thread")]
async fn post_tool_use_policy_audit(world: HookPolicyWorld) {
    let _ = world;
}
