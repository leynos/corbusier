//! Given steps for handoff BDD scenarios.

use super::world::{HandoffWorld, create_and_store_session, create_tool_call_refs, run_async};
use corbusier::message::domain::{HandoffSessionParams, SequenceNumber, TurnId};
use corbusier::message::ports::agent_session::AgentSessionRepository;
use corbusier::message::services::ServiceInitiateParams;
use eyre::{WrapErr, eyre};
use rstest_bdd_macros::given;

// ============================================================================
// Background Steps
// ============================================================================

#[given("an active agent session for a conversation")]
fn active_agent_session(world: &mut HandoffWorld) -> Result<(), eyre::Report> {
    let session = create_and_store_session(world, "source-agent", SequenceNumber::new(1))?;
    world.source_session = Some(session);
    Ok(())
}

// ============================================================================
// Given Steps
// ============================================================================

#[given("an initiated handoff to a target agent")]
fn initiated_handoff(world: &mut HandoffWorld) -> Result<(), eyre::Report> {
    if world.source_session.is_none() {
        active_agent_session(world)?;
    }

    let source = world
        .source_session
        .as_ref()
        .ok_or_else(|| eyre!("no source session"))?;

    let params = ServiceInitiateParams::new(
        source.session_id,
        "target-agent",
        world.prior_turn_id,
        SequenceNumber::new(5),
    )
    .with_reason("task requires specialist");
    let handoff = run_async(world.service.initiate(params)).wrap_err("initiate handoff")?;

    world.current_handoff = Some(handoff);
    Ok(())
}

#[given("a conversation with tool calls")]
fn conversation_with_tool_calls(world: &mut HandoffWorld) -> Result<(), eyre::Report> {
    active_agent_session(world)?;

    world.tool_call_refs = create_tool_call_refs();

    Ok(())
}

#[given("a completed handoff from agent A to agent B")]
fn completed_handoff_a_to_b(world: &mut HandoffWorld) -> Result<(), eyre::Report> {
    let agent_a = create_and_store_session(world, "agent-A", SequenceNumber::new(1))?;

    let initiate_params = ServiceInitiateParams::new(
        agent_a.session_id,
        "agent-B",
        TurnId::new(),
        SequenceNumber::new(5),
    );
    let handoff = run_async(world.service.initiate(initiate_params)).wrap_err("initiate A->B")?;

    let params = HandoffSessionParams::new(
        world.conversation_id,
        "agent-B",
        SequenceNumber::new(6),
        handoff.handoff_id,
    );
    let agent_b =
        run_async(world.service.create_target_session(params)).wrap_err("create agent B")?;

    run_async(world.service.complete(
        handoff.handoff_id,
        agent_b.session_id,
        SequenceNumber::new(6),
    ))
    .wrap_err("complete A->B")?;

    world.source_session = Some(agent_b);
    Ok(())
}
