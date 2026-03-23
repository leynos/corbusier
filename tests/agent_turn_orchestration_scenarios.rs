//! Behaviour tests for agent turn orchestration and session lifecycle.

mod agent_turn_orchestration_steps;

use agent_turn_orchestration_steps::world::{AgentTurnWorld, world};
use rstest_bdd_macros::scenario;

#[scenario(
    path = "tests/features/agent_turn_orchestration.feature",
    name = "Execute a turn with routed tool calls"
)]
#[tokio::test(flavor = "multi_thread")]
async fn execute_turn_with_routed_tools(world: AgentTurnWorld) {
    let _ = world;
}

#[scenario(
    path = "tests/features/agent_turn_orchestration.feature",
    name = "Reuse an active session before expiry"
)]
#[tokio::test(flavor = "multi_thread")]
async fn reuse_session_before_expiry(world: AgentTurnWorld) {
    let _ = world;
}

#[scenario(
    path = "tests/features/agent_turn_orchestration.feature",
    name = "Rotate a session when it is expired"
)]
#[tokio::test(flavor = "multi_thread")]
async fn rotate_expired_session(world: AgentTurnWorld) {
    let _ = world;
}

#[scenario(
    path = "tests/features/agent_turn_orchestration.feature",
    name = "Surface tool routing failure"
)]
#[tokio::test(flavor = "multi_thread")]
async fn surface_tool_routing_failure(world: AgentTurnWorld) {
    let _ = world;
}

#[scenario(
    path = "tests/features/agent_turn_orchestration.feature",
    name = "Concurrent turns on same backend/conversation"
)]
#[tokio::test(flavor = "multi_thread")]
async fn concurrent_turns_on_same_backend_conversation(world: AgentTurnWorld) {
    let _ = world;
}
