//! Then steps for agent turn orchestration BDD scenarios.

use super::world::{AgentTurnWorld, AuditStatusLabel, ConversationLabel};
use corbusier::agent_backend::{
    domain::{ToolCallAuditStatus, TurnSession, TurnSessionId, TurnSessionStatus},
    ports::{SessionSlotKey, TurnSessionRepository},
    services::{AgentTurnOrchestrationError, ExecuteAgentTurnResponse},
};
use rstest_bdd_macros::then;

fn successful_response(world: &AgentTurnWorld) -> Result<&ExecuteAgentTurnResponse, eyre::Report> {
    let result = world
        .last_result
        .as_ref()
        .ok_or_else(|| eyre::eyre!("missing turn result in world"))?;
    result
        .as_ref()
        .map_err(|err| eyre::eyre!("expected success, got {err}"))
}

fn expect_existing_session_id(world: &AgentTurnWorld) -> Result<TurnSessionId, eyre::Report> {
    world
        .existing_session_id
        .ok_or_else(|| eyre::eyre!("missing existing session in world"))
}

#[derive(Debug, Clone, Copy)]
enum SessionOutcome {
    Reused,
    Rotated,
}

fn assert_session_outcome(
    world: &AgentTurnWorld,
    outcome: SessionOutcome,
) -> Result<(), eyre::Report> {
    let prior_session_id = expect_existing_session_id(world)?;
    let response = successful_response(world)?;
    match outcome {
        SessionOutcome::Reused => {
            if !response.reused_session() {
                return Err(eyre::eyre!("expected reused_session=true"));
            }
            if response.session_id() != prior_session_id {
                return Err(eyre::eyre!(
                    "expected session {:?}, got {:?}",
                    prior_session_id,
                    response.session_id()
                ));
            }
        }
        SessionOutcome::Rotated => {
            if !response.rotated_session() {
                return Err(eyre::eyre!("expected rotated_session=true"));
            }
            if response.session_id() == prior_session_id {
                return Err(eyre::eyre!("expected a new rotated session id"));
            }
        }
    }
    Ok(())
}

#[then("the turn succeeds")]
fn turn_succeeds(world: &AgentTurnWorld) -> Result<(), eyre::Report> {
    successful_response(world)?;
    Ok(())
}

#[then("one tool result is returned")]
fn one_tool_result_returned(world: &AgentTurnWorld) -> Result<(), eyre::Report> {
    let response = successful_response(world)?;
    if response.tool_results().len() != 1 {
        return Err(eyre::eyre!(
            "expected one tool result, got {}",
            response.tool_results().len()
        ));
    }
    Ok(())
}

#[then(r#"all tool audits are "{status}""#)]
fn all_tool_audits_are(
    world: &AgentTurnWorld,
    status: AuditStatusLabel,
) -> Result<(), eyre::Report> {
    let response = successful_response(world)?;

    let expected = match status.as_str() {
        "succeeded" => ToolCallAuditStatus::Succeeded,
        "failed" => ToolCallAuditStatus::Failed,
        other => return Err(eyre::eyre!("unsupported audit status assertion: {other}")),
    };

    let audits = response.tool_call_audits();
    if audits.is_empty() {
        return Err(eyre::eyre!("expected at least one tool audit"));
    }
    if !audits.iter().all(|audit| audit.status() == expected) {
        let status_label = status.as_str();
        return Err(eyre::eyre!(
            "expected all audits to be {status_label}, got {:?}",
            audits
        ));
    }
    Ok(())
}

#[then("the existing session is reused")]
fn existing_session_reused(world: &AgentTurnWorld) -> Result<(), eyre::Report> {
    assert_session_outcome(world, SessionOutcome::Reused)
}

#[then("the session is rotated")]
fn session_rotated(world: &AgentTurnWorld) -> Result<(), eyre::Report> {
    assert_session_outcome(world, SessionOutcome::Rotated)
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
    conversation: ConversationLabel,
) -> Result<(), eyre::Report> {
    let conversation_label = conversation.0;
    let backend_id = world
        .backend_id
        .ok_or_else(|| eyre::eyre!("missing backend id in world"))?;
    let conversation_id = world
        .conversations
        .get(&conversation_label)
        .copied()
        .ok_or_else(|| eyre::eyre!("missing conversation id for label {conversation_label}"))?;

    let active = super::world::run_async(
        world
            .session_repository
            .find_active_session(&world.ctx, SessionSlotKey::new(backend_id, conversation_id)),
    )?;
    if active.is_none() {
        return Err(eyre::eyre!("expected one active session but found none"));
    }

    let sessions = world.session_repository.all_sessions()?;
    let active_count = sessions
        .iter()
        .filter(|session: &&TurnSession| session.backend_id() == backend_id)
        .filter(|session: &&TurnSession| session.conversation_id() == conversation_id)
        .filter(|session: &&TurnSession| session.status() == TurnSessionStatus::Active)
        .count();
    if active_count != 1 {
        return Err(eyre::eyre!(
            "expected one active session, found {active_count}"
        ));
    }
    Ok(())
}
