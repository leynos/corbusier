//! Given steps for agent turn orchestration BDD scenarios.

use super::world::{
    AgentTurnWorld, AssistantText, BackendNameLabel, ConversationLabel, ToolName, run_async,
};
use chrono::{Duration, Utc};
use corbusier::agent_backend::{
    domain::{
        AgentBackendRegistration, AgentCapabilities, BackendInfo, BackendName,
        PersistedTurnSessionData, RuntimeSessionId, ToolCallRequest, TurnExecutionResult,
        TurnSession, TurnSessionCreateParams, TurnSessionId, TurnSessionStatus,
    },
    ports::{BackendRegistryRepository, TurnSessionRepository},
};
use rstest_bdd_macros::given;
use serde_json::json;

fn queue_turn_result_without_tools(
    world: &mut AgentTurnWorld,
    assistant_text: impl Into<String>,
) -> Result<(), eyre::Report> {
    world
        .runtime
        .queue_turn_result(TurnExecutionResult::new(assistant_text, Vec::new()))?;
    Ok(())
}

fn seeded_session(world: &mut AgentTurnWorld, session: &TurnSession) -> Result<(), eyre::Report> {
    world.existing_session_id = Some(session.id());
    run_async(world.session_repository.upsert_session(&world.ctx, session))?;
    Ok(())
}

fn queue_tool_call_result(
    world: &mut AgentTurnWorld,
    assistant_text: &str,
    tool_name: &str,
) -> Result<(), eyre::Report> {
    let result = TurnExecutionResult::new(
        assistant_text,
        vec![ToolCallRequest::new(tool_name, json!({"key": "value"}))?],
    );
    world.runtime.queue_turn_result(result)?;
    Ok(())
}

#[given(r#"an active backend named "{name}""#)]
fn an_active_backend_named(
    world: &mut AgentTurnWorld,
    name: BackendNameLabel,
) -> Result<(), eyre::Report> {
    let backend_name = BackendName::new(&name.0)?;
    let capabilities = AgentCapabilities::new(true, true);
    let info = BackendInfo::new(name.0, "1.0.0", "bdd-provider")?;
    let registration =
        AgentBackendRegistration::new(backend_name, capabilities, info, &mockable::DefaultClock);
    world.backend_id = Some(registration.id());
    run_async(world.backend_registry.register(&world.ctx, &registration))?;
    Ok(())
}

#[given(r#"the runtime returns assistant text "{text}" with tool "{tool}""#)]
fn runtime_returns_text_with_tool(
    world: &mut AgentTurnWorld,
    text: AssistantText,
    tool: ToolName,
) -> Result<(), eyre::Report> {
    queue_tool_call_result(world, &text.0, &tool.0)
}

#[given(r#"the runtime returns assistant text "{text}" with no tools"#)]
fn runtime_returns_text_with_no_tools(
    world: &mut AgentTurnWorld,
    text: AssistantText,
) -> Result<(), eyre::Report> {
    queue_turn_result_without_tools(world, text.0)
}

#[given(r#"the runtime returns assistant texts "{first_text}" and "{second_text}" with no tools"#)]
fn runtime_returns_two_texts_with_no_tools(
    world: &mut AgentTurnWorld,
    first_text: AssistantText,
    second_text: AssistantText,
) -> Result<(), eyre::Report> {
    queue_turn_result_without_tools(world, first_text.0)?;
    queue_turn_result_without_tools(world, second_text.0)?;
    Ok(())
}

#[given(r#"an existing active session for conversation "{conversation}""#)]
fn existing_active_session(
    world: &mut AgentTurnWorld,
    conversation: ConversationLabel,
) -> Result<(), eyre::Report> {
    let backend_id = world
        .backend_id
        .ok_or_else(|| eyre::eyre!("backend must be registered first"))?;
    let conversation_id = world.conversation_id(&conversation.0);
    let session = TurnSession::new(TurnSessionCreateParams {
        backend_id,
        conversation_id,
        runtime_session_id: RuntimeSessionId::new("existing-runtime-session")?,
        ttl: Duration::minutes(5),
        now: Utc::now(),
    })?;
    seeded_session(world, &session)
}

#[given(r#"an expired active session for conversation "{conversation}""#)]
fn expired_active_session(
    world: &mut AgentTurnWorld,
    conversation: ConversationLabel,
) -> Result<(), eyre::Report> {
    let backend_id = world
        .backend_id
        .ok_or_else(|| eyre::eyre!("backend must be registered first"))?;
    let conversation_id = world.conversation_id(&conversation.0);
    let now = Utc::now();

    let session = TurnSession::from_persisted(PersistedTurnSessionData {
        id: TurnSessionId::new(),
        backend_id,
        conversation_id,
        runtime_session_id: RuntimeSessionId::new("expired-runtime-session")?,
        status: TurnSessionStatus::Active,
        ttl_seconds: 30,
        started_at: now - Duration::seconds(90),
        last_used_at: now - Duration::seconds(90),
        expires_at: now - Duration::seconds(1),
        ended_at: None,
        turn_count: 2,
    });
    seeded_session(world, &session)
}

#[given(r#"the tool router fails for tool "{tool}""#)]
fn tool_router_fails(world: &mut AgentTurnWorld, tool: ToolName) -> Result<(), eyre::Report> {
    world
        .tool_router
        .fail_tool(tool.0, "bdd configured failure")?;
    Ok(())
}
