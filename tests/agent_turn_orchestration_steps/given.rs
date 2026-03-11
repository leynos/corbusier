//! Given steps for agent turn orchestration BDD scenarios.

use super::world::{AgentTurnWorld, run_async};
use chrono::{Duration, Utc};
use corbusier::agent_backend::{
    domain::{
        AgentBackendRegistration, AgentCapabilities, BackendInfo, BackendName,
        PersistedTurnSessionData, RuntimeSessionId, ToolCallRequest, TurnExecutionResult,
        TurnSession, TurnSessionCreateParams, TurnSessionStatus,
    },
    ports::{BackendRegistryRepository, TurnSessionRepository},
};
use rstest_bdd_macros::given;
use serde_json::json;

#[given(r#"an active backend named "{name}""#)]
fn an_active_backend_named(world: &mut AgentTurnWorld, name: String) -> Result<(), eyre::Report> {
    let backend_name = BackendName::new(name.clone())?;
    let capabilities = AgentCapabilities::new(true, true);
    let info = BackendInfo::new(name, "1.0.0", "bdd-provider")?;
    let registration =
        AgentBackendRegistration::new(backend_name, capabilities, info, &mockable::DefaultClock);
    world.backend_id = Some(registration.id());
    run_async(world.backend_registry.register(&world.ctx, &registration))?;
    Ok(())
}

#[given(r#"the runtime returns assistant text "{text}" with tool "{tool}""#)]
fn runtime_returns_text_with_tool(
    world: &mut AgentTurnWorld,
    text: String,
    tool: String,
) -> Result<(), eyre::Report> {
    let result = TurnExecutionResult::new(
        text,
        vec![ToolCallRequest::new(tool, json!({"key": "value"}))?],
    );
    world.runtime.queue_turn_result(result)?;
    Ok(())
}

#[given(r#"the runtime returns assistant text "{text}" with no tools"#)]
fn runtime_returns_text_with_no_tools(
    world: &mut AgentTurnWorld,
    text: String,
) -> Result<(), eyre::Report> {
    world
        .runtime
        .queue_turn_result(TurnExecutionResult::new(text, Vec::new()))?;
    Ok(())
}

#[given(r#"an existing active session for conversation "{conversation}""#)]
fn existing_active_session(
    world: &mut AgentTurnWorld,
    conversation: String,
) -> Result<(), eyre::Report> {
    let backend_id = world
        .backend_id
        .ok_or_else(|| eyre::eyre!("backend must be registered first"))?;
    let conversation_id = world.conversation_id(&conversation);
    let session = TurnSession::new(TurnSessionCreateParams {
        backend_id,
        conversation_id,
        runtime_session_id: RuntimeSessionId::new("existing-runtime-session")?,
        ttl: Duration::minutes(5),
        now: Utc::now(),
    })?;
    world.existing_session_id = Some(session.id());
    run_async(world.session_repository.upsert_session(&session))?;
    Ok(())
}

#[given(r#"an expired active session for conversation "{conversation}""#)]
fn expired_active_session(
    world: &mut AgentTurnWorld,
    conversation: String,
) -> Result<(), eyre::Report> {
    let backend_id = world
        .backend_id
        .ok_or_else(|| eyre::eyre!("backend must be registered first"))?;
    let conversation_id = world.conversation_id(&conversation);
    let now = Utc::now();

    let session = TurnSession::from_persisted(PersistedTurnSessionData {
        id: corbusier::agent_backend::domain::TurnSessionId::new(),
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

    world.existing_session_id = Some(session.id());
    run_async(world.session_repository.upsert_session(&session))?;
    Ok(())
}

#[given(r#"the tool router fails for tool "{tool}""#)]
fn tool_router_fails(world: &mut AgentTurnWorld, tool: String) -> Result<(), eyre::Report> {
    world
        .tool_router
        .fail_tool(tool, "bdd configured failure")?;
    Ok(())
}
