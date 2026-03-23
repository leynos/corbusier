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
    world.last_result = Some(run_async(world.service.execute_turn(&world.ctx, request)));
    Ok(())
}

#[when(r#"I execute two concurrent turns for conversation "{conversation}""#)]
fn execute_two_concurrent_turns_for_conversation(
    world: &mut AgentTurnWorld,
    conversation: String,
) -> Result<(), eyre::Report> {
    let backend_id = world
        .backend_id
        .ok_or_else(|| eyre::eyre!("backend must be registered first"))?;
    let conversation_id = world.conversation_id(&conversation);
    let first = ExecuteAgentTurnRequest::new(
        backend_id,
        TurnExecutionRequest::new(conversation_id, "bdd prompt one", Vec::new()),
    );
    let second = ExecuteAgentTurnRequest::new(
        backend_id,
        TurnExecutionRequest::new(conversation_id, "bdd prompt two", Vec::new()),
    );

    world.concurrent_results = Some(run_async(async {
        tokio::join!(
            world.service.execute_turn(&world.ctx, first),
            world.service.execute_turn(&world.ctx, second)
        )
    }));
    Ok(())
}
