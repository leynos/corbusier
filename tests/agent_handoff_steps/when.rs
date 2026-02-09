//! When steps for handoff BDD scenarios.

use super::world::{HandoffWorld, run_async};
use corbusier::message::domain::{HandoffSessionParams, SequenceNumber, TurnId};
use corbusier::message::services::ServiceInitiateParams;
use eyre::{WrapErr, eyre};
use rstest_bdd_macros::when;

// ============================================================================
// When Steps
// ============================================================================

#[when("the current agent initiates a handoff to a specialist agent")]
fn initiate_specialist_handoff(world: &mut HandoffWorld) -> Result<(), eyre::Report> {
    let source = world
        .source_session
        .as_ref()
        .ok_or_else(|| eyre!("no source session"))?;

    let initiate_params = ServiceInitiateParams::new(
        source.session_id,
        "specialist-agent",
        world.prior_turn_id,
        SequenceNumber::new(5),
    )
    .with_reason("task requires specialist");
    let handoff =
        run_async(world.service.initiate(initiate_params)).wrap_err("initiate handoff")?;

    world.current_handoff = Some(handoff);
    Ok(())
}

#[when("the target agent creates a new session")]
fn target_creates_session(world: &mut HandoffWorld) -> Result<(), eyre::Report> {
    let handoff = world
        .current_handoff
        .as_ref()
        .ok_or_else(|| eyre!("no current handoff"))?;

    let params = HandoffSessionParams::new(
        world.conversation_id,
        "target-agent",
        SequenceNumber::new(10),
        handoff.handoff_id,
    );
    let target =
        run_async(world.service.create_target_session(params)).wrap_err("create target session")?;

    world.target_session = Some(target);
    Ok(())
}

#[when("the handoff is completed")]
fn complete_handoff(world: &mut HandoffWorld) -> Result<(), eyre::Report> {
    let handoff = world
        .current_handoff
        .as_ref()
        .ok_or_else(|| eyre!("no current handoff"))?;

    let target = world
        .target_session
        .as_ref()
        .ok_or_else(|| eyre!("no target session"))?;

    let completed = run_async(world.service.complete(
        handoff.handoff_id,
        target.session_id,
        SequenceNumber::new(10),
    ))
    .wrap_err("complete handoff")?;

    world.current_handoff = Some(completed);
    Ok(())
}

#[when("the handoff is cancelled")]
fn cancel_handoff(world: &mut HandoffWorld) -> Result<(), eyre::Report> {
    let handoff = world
        .current_handoff
        .as_ref()
        .ok_or_else(|| eyre!("no current handoff"))?;

    run_async(
        world
            .service
            .cancel(handoff.handoff_id, Some("target unavailable")),
    )
    .wrap_err("cancel handoff")?;

    Ok(())
}

#[when("a handoff is initiated with tool call references")]
fn initiate_with_tool_calls(world: &mut HandoffWorld) -> Result<(), eyre::Report> {
    let source = world
        .source_session
        .as_ref()
        .ok_or_else(|| eyre!("no source session"))?;

    let initiate_params = ServiceInitiateParams::new(
        source.session_id,
        "specialist-agent",
        world.prior_turn_id,
        SequenceNumber::new(5),
    )
    .with_reason("tool results need review");
    let mut handoff =
        run_async(world.service.initiate(initiate_params)).wrap_err("initiate handoff")?;

    for tcr in &world.tool_call_refs {
        handoff = handoff.with_triggering_tool_call(tcr.clone());
    }

    world.current_handoff = Some(handoff);
    Ok(())
}

#[when("agent B initiates a handoff to agent C")]
fn agent_b_to_c(world: &mut HandoffWorld) -> Result<(), eyre::Report> {
    let agent_b = world
        .source_session
        .as_ref()
        .ok_or_else(|| eyre!("no agent B session"))?;

    let initiate_params = ServiceInitiateParams::new(
        agent_b.session_id,
        "agent-C",
        TurnId::new(),
        SequenceNumber::new(10),
    )
    .with_reason("need domain expert");
    let handoff = run_async(world.service.initiate(initiate_params)).wrap_err("initiate B->C")?;

    let params = HandoffSessionParams::new(
        world.conversation_id,
        "agent-C",
        SequenceNumber::new(11),
        handoff.handoff_id,
    );
    let agent_c =
        run_async(world.service.create_target_session(params)).wrap_err("create agent C")?;

    let completed = run_async(world.service.complete(
        handoff.handoff_id,
        agent_c.session_id,
        SequenceNumber::new(11),
    ))
    .wrap_err("complete B->C")?;

    world.target_session = Some(agent_c);
    world.current_handoff = Some(completed);
    Ok(())
}
