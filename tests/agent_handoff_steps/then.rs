//! Then steps for handoff BDD scenarios.

use super::world::{HandoffWorld, run_async};
use corbusier::message::domain::{
    AgentSession, AgentSessionId, AgentSessionState, HandoffMetadata, HandoffStatus, SnapshotType,
};
use corbusier::message::ports::{
    agent_session::AgentSessionRepository, context_snapshot::ContextSnapshotPort,
    handoff::AgentHandoffPort,
};
use eyre::{WrapErr, eyre};
use rstest_bdd_macros::then;

// ============================================================================
// Helper Functions
// ============================================================================

/// Extract the current handoff from world, returning an error if not present.
fn get_current_handoff(world: &HandoffWorld) -> Result<&HandoffMetadata, eyre::Report> {
    world
        .current_handoff
        .as_ref()
        .ok_or_else(|| eyre!("no handoff"))
}

/// Extract the source session from world, returning an error if not present.
fn get_source_session(world: &HandoffWorld) -> Result<&AgentSession, eyre::Report> {
    world
        .source_session
        .as_ref()
        .ok_or_else(|| eyre!("no source session"))
}

/// Fetch the latest version of a session from the repository.
fn fetch_session(
    world: &HandoffWorld,
    session_id: AgentSessionId,
) -> Result<AgentSession, eyre::Report> {
    run_async(world.session_repo.find_by_id(session_id))
        .wrap_err("find source")?
        .ok_or_else(|| eyre!("source session not found"))
}

/// Assert a condition, returning an error with the given message if false.
fn assert_that(condition: bool, message: impl Into<String>) -> Result<(), eyre::Report> {
    if condition {
        Ok(())
    } else {
        Err(eyre!(message.into()))
    }
}

// ============================================================================
// Then Steps
// ============================================================================

#[then("a handoff record is created with initiated status")]
fn handoff_initiated_status(world: &HandoffWorld) -> Result<(), eyre::Report> {
    let handoff = get_current_handoff(world)?;
    assert_that(
        handoff.status == HandoffStatus::Initiated,
        format!("expected Initiated status, got {:?}", handoff.status),
    )
}

#[then("a context snapshot is captured for the source session")]
fn snapshot_captured(world: &HandoffWorld) -> Result<(), eyre::Report> {
    let source = world
        .source_session
        .as_ref()
        .ok_or_else(|| eyre!("no source session"))?;

    let snapshots = run_async(
        world
            .snapshot_adapter
            .find_snapshots_for_session(source.session_id),
    )
    .wrap_err("find snapshots")?;

    if snapshots.is_empty() {
        return Err(eyre!("no snapshots captured"));
    }

    let has_handoff_snapshot = snapshots
        .iter()
        .any(|s| s.snapshot_type == SnapshotType::HandoffInitiated);

    if !has_handoff_snapshot {
        return Err(eyre!("no HandoffInitiated snapshot found"));
    }

    Ok(())
}

#[then("the source session is marked as handed off")]
fn source_handed_off(world: &HandoffWorld) -> Result<(), eyre::Report> {
    let source = get_source_session(world)?;
    let session = fetch_session(world, source.session_id)?;
    assert_that(
        session.state == AgentSessionState::HandedOff,
        format!("expected HandedOff state, got {:?}", session.state),
    )
}

#[then("the handoff record links source and target sessions")]
fn handoff_links_sessions(world: &HandoffWorld) -> Result<(), eyre::Report> {
    let handoff = world
        .current_handoff
        .as_ref()
        .ok_or_else(|| eyre!("no handoff"))?;

    let target = world
        .target_session
        .as_ref()
        .ok_or_else(|| eyre!("no target session"))?;

    if handoff.target_session_id != Some(target.session_id) {
        return Err(eyre!("handoff does not link to target session"));
    }

    Ok(())
}

#[then("the handoff status is completed")]
fn handoff_completed(world: &HandoffWorld) -> Result<(), eyre::Report> {
    let handoff = world
        .current_handoff
        .as_ref()
        .ok_or_else(|| eyre!("no handoff"))?;

    if handoff.status != HandoffStatus::Completed {
        return Err(eyre!("expected Completed status, got {:?}", handoff.status));
    }

    if handoff.completed_at.is_none() {
        return Err(eyre!("completed_at should be set"));
    }

    Ok(())
}

#[then("the source session is reverted to active state")]
fn source_reverted(world: &HandoffWorld) -> Result<(), eyre::Report> {
    let source = get_source_session(world)?;
    let session = fetch_session(world, source.session_id)?;
    assert_that(
        session.state == AgentSessionState::Active,
        format!("expected Active state, got {:?}", session.state),
    )?;
    assert_that(
        session.terminated_by_handoff.is_none(),
        "terminated_by_handoff should be None",
    )
}

#[then("no target session is created")]
fn no_target_session(world: &HandoffWorld) -> Result<(), eyre::Report> {
    if world.target_session.is_some() {
        return Err(eyre!("target session should not exist"));
    }
    Ok(())
}

#[then("the handoff metadata includes the prior turn id")]
fn handoff_includes_turn(world: &HandoffWorld) -> Result<(), eyre::Report> {
    let handoff = get_current_handoff(world)?;
    assert_that(
        handoff.prior_turn_id == world.prior_turn_id,
        "prior_turn_id does not match",
    )
}

#[then("the handoff metadata includes the triggering tool calls")]
fn handoff_includes_tool_calls(world: &HandoffWorld) -> Result<(), eyre::Report> {
    let handoff = world
        .current_handoff
        .as_ref()
        .ok_or_else(|| eyre!("no handoff"))?;

    if handoff.triggering_tool_calls.len() != world.tool_call_refs.len() {
        return Err(eyre!(
            "expected {} tool calls, got {}",
            world.tool_call_refs.len(),
            handoff.triggering_tool_calls.len()
        ));
    }

    Ok(())
}

#[then("the conversation history shows all agent sessions")]
fn history_shows_all_sessions(world: &HandoffWorld) -> Result<(), eyre::Report> {
    let sessions = run_async(
        world
            .session_repo
            .find_by_conversation(world.conversation_id),
    )
    .wrap_err("list sessions")?;

    if sessions.len() < 3 {
        return Err(eyre!(
            "expected at least 3 sessions, got {}",
            sessions.len()
        ));
    }

    let agents: Vec<&str> = sessions.iter().map(|s| s.agent_backend.as_str()).collect();

    for expected in &["agent-A", "agent-B", "agent-C"] {
        if !agents.contains(expected) {
            return Err(eyre!("missing agent: {expected}"));
        }
    }

    Ok(())
}

#[then("each handoff is linked in sequence")]
fn handoffs_linked_in_sequence(world: &HandoffWorld) -> Result<(), eyre::Report> {
    let handoffs = run_async(
        world
            .handoff_adapter
            .list_handoffs_for_conversation(world.conversation_id),
    )
    .wrap_err("list handoffs")?;

    if handoffs.len() != 2 {
        return Err(eyre!("expected 2 handoffs, got {}", handoffs.len()));
    }

    for handoff in &handoffs {
        if handoff.status != HandoffStatus::Completed {
            return Err(eyre!("handoff {:?} is not completed", handoff.handoff_id));
        }
    }

    Ok(())
}
