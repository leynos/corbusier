//! When steps for agent turn orchestration BDD scenarios.

use super::world::{AgentTurnWorld, run_async};
use corbusier::agent_backend::{domain::TurnExecutionRequest, services::ExecuteAgentTurnRequest};
use rstest_bdd_macros::when;

#[when(r#"I execute a turn for conversation "{conversation}""#)]
fn execute_turn_for_conversation(
    world: &mut AgentTurnWorld,
    conversation: String,
) -> Result<(), eyre::Report> {
    let backend_id = world
        .backend_id
        .ok_or_else(|| eyre::eyre!("backend must be registered first"))?;
    let conversation_id = world.conversation_id(&conversation);
    let request = ExecuteAgentTurnRequest::new(
        backend_id,
        TurnExecutionRequest::new(conversation_id, "bdd prompt", Vec::new()),
    );
    world.last_result = Some(run_async(world.service.execute_turn(request)));
    Ok(())
}
