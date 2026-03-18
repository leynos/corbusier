//! Then steps for agent turn orchestration BDD scenarios.

use super::world::AgentTurnWorld;
use corbusier::agent_backend::{
    domain::ToolCallAuditStatus, services::AgentTurnOrchestrationError,
};
use corbusier::agent_backend::{domain::TurnSessionStatus, ports::{SessionSlotKey, TurnSessionRepository}};
use rstest_bdd_macros::then;

#[then("the turn succeeds")]
fn turn_succeeds(world: &AgentTurnWorld) -> Result<(), eyre::Report> {
    let result = world
        .last_result
        .as_ref()
        .ok_or_else(|| eyre::eyre!("missing turn result in world"))?;
    if result.is_err() {
        return Err(eyre::eyre!("expected successful turn, got {result:?}"));
    }
    Ok(())
}

#[then("one tool result is returned")]
fn one_tool_result_returned(world: &AgentTurnWorld) -> Result<(), eyre::Report> {
    let result = world
        .last_result
        .as_ref()
        .ok_or_else(|| eyre::eyre!("missing turn result in world"))?;
    let response = result
        .as_ref()
        .map_err(|err| eyre::eyre!("expected success, got {err}"))?;
    if response.tool_results().len() != 1 {
        return Err(eyre::eyre!(
            "expected one tool result, got {}",
            response.tool_results().len()
        ));
    }
    Ok(())
}

#[then(r#"all tool audits are "{status}""#)]
fn all_tool_audits_are(world: &AgentTurnWorld, status: String) -> Result<(), eyre::Report> {
    let result = world
        .last_result
        .as_ref()
        .ok_or_else(|| eyre::eyre!("missing turn result in world"))?;
    let response = result
        .as_ref()
        .map_err(|err| eyre::eyre!("expected success, got {err}"))?;

    let expected = match status.as_str() {
        "succeeded" => ToolCallAuditStatus::Succeeded,
        other => return Err(eyre::eyre!("unsupported audit status assertion: {other}")),
    };

    if !response
        .tool_call_audits()
        .iter()
        .all(|audit| audit.status() == expected)
    {
        return Err(eyre::eyre!(
            "expected all audits to be {status}, got {:?}",
            response.tool_call_audits()
        ));
    }
    Ok(())
}

#[then("the existing session is reused")]
fn existing_session_reused(world: &AgentTurnWorld) -> Result<(), eyre::Report> {
    let expected_session_id = world
        .existing_session_id
        .ok_or_else(|| eyre::eyre!("missing existing session in world"))?;
    let result = world
        .last_result
        .as_ref()
        .ok_or_else(|| eyre::eyre!("missing turn result in world"))?;
    let response = result
        .as_ref()
        .map_err(|err| eyre::eyre!("expected success, got {err}"))?;

    if !response.reused_session() {
        return Err(eyre::eyre!("expected reused_session=true"));
    }
    if response.session_id() != expected_session_id {
        return Err(eyre::eyre!(
            "expected session {:?}, got {:?}",
            expected_session_id,
            response.session_id()
        ));
    }
    Ok(())
}

#[then("the session is rotated")]
fn session_rotated(world: &AgentTurnWorld) -> Result<(), eyre::Report> {
    let expected_prior_session_id = world
        .existing_session_id
        .ok_or_else(|| eyre::eyre!("missing existing session in world"))?;
    let result = world
        .last_result
        .as_ref()
        .ok_or_else(|| eyre::eyre!("missing turn result in world"))?;
    let response = result
        .as_ref()
        .map_err(|err| eyre::eyre!("expected success, got {err}"))?;

    if !response.rotated_session() {
        return Err(eyre::eyre!("expected rotated_session=true"));
    }
    if response.session_id() == expected_prior_session_id {
        return Err(eyre::eyre!("expected a new rotated session id"));
    }
    Ok(())
}

#[then("the turn fails with a tool routing error")]
fn turn_fails_with_tool_routing_error(world: &AgentTurnWorld) -> Result<(), eyre::Report> {
    let result = world
        .last_result
        .as_ref()
        .ok_or_else(|| eyre::eyre!("missing turn result in world"))?;
    if !matches!(result, Err(AgentTurnOrchestrationError::ToolRouting { .. })) {
        return Err(eyre::eyre!("expected tool routing failure, got {result:?}"));
    }
    Ok(())
}

#[then("both concurrent turns succeed")]
fn both_concurrent_turns_succeed(world: &AgentTurnWorld) -> Result<(), eyre::Report> {
    let (first, second) = world
        .concurrent_results
        .as_ref()
        .ok_or_else(|| eyre::eyre!("missing concurrent turn results in world"))?;
    if first.is_err() || second.is_err() {
        return Err(eyre::eyre!(
            "expected both concurrent turns to succeed, got ({first:?}, {second:?})"
        ));
    }
    Ok(())
}

#[then(r#"only one active session remains for conversation "{conversation}""#)]
fn only_one_active_session_remains_for_conversation(
    world: &AgentTurnWorld,
    conversation: String,
) -> Result<(), eyre::Report> {
    let backend_id = world
        .backend_id
        .ok_or_else(|| eyre::eyre!("missing backend id in world"))?;
    let conversation_id = world
        .conversations
        .get(&conversation)
        .copied()
        .ok_or_else(|| eyre::eyre!("missing conversation id for label {conversation}"))?;

    let active = super::world::run_async(world.session_repository.find_active_session(
        &world.ctx,
        SessionSlotKey::new(backend_id, conversation_id),
    ))?;
    if active.is_none() {
        return Err(eyre::eyre!("expected one active session but found none"));
    }

    let sessions = world.session_repository.all_sessions()?;
    let active_count = sessions
        .iter()
        .filter(|session| session.backend_id() == backend_id)
        .filter(|session| session.conversation_id() == conversation_id)
        .filter(|session| session.status() == TurnSessionStatus::Active)
        .count();
    if active_count != 1 {
        return Err(eyre::eyre!(
            "expected one active session, found {active_count}"
        ));
    }
    Ok(())
}
