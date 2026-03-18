//! Repository-focused orchestration persistence tests against `PostgreSQL`.

use chrono::{Duration, Utc};
use corbusier::agent_backend::{
    domain::{RuntimeSessionId, TurnSession, TurnSessionCreateParams},
    ports::{SessionSlotKey, TurnSessionRepository, TurnSessionRepositoryError},
};
use rstest::rstest;
use uuid::Uuid;

use super::common::{
    OrchestrationContext, context, ensure_conversation_exists, other_tenant_context,
    register_backend,
};
use crate::postgres::helpers::BoxError;

#[rstest]
#[tokio::test(flavor = "multi_thread")]
async fn postgres_detects_concurrent_active_session_conflict(
    #[future] context: Result<OrchestrationContext, BoxError>,
) -> Result<(), BoxError> {
    let ctx = context.await?;
    let backend_id = corbusier::agent_backend::domain::BackendId::from_uuid(
        register_backend(&ctx, "claude_code_sdk").await?,
    );
    let conversation_id = Uuid::new_v4();
    ensure_conversation_exists(&ctx, conversation_id).await?;

    let now = Utc::now();
    let session_a = TurnSession::new(TurnSessionCreateParams {
        backend_id,
        conversation_id,
        runtime_session_id: RuntimeSessionId::new("runtime-a")
            .map_err(|err| Box::new(err) as BoxError)?,
        ttl: Duration::minutes(5),
        now,
    })
    .map_err(|err| Box::new(err) as BoxError)?;
    let session_b = TurnSession::new(TurnSessionCreateParams {
        backend_id,
        conversation_id,
        runtime_session_id: RuntimeSessionId::new("runtime-b")
            .map_err(|err| Box::new(err) as BoxError)?,
        ttl: Duration::minutes(5),
        now,
    })
    .map_err(|err| Box::new(err) as BoxError)?;

    let (first, second) = tokio::join!(
        ctx.session_repository.upsert_session(&ctx.ctx, &session_a),
        ctx.session_repository.upsert_session(&ctx.ctx, &session_b)
    );

    let first_conflict = matches!(
        &first,
        Err(TurnSessionRepositoryError::ActiveSessionConflict { .. })
    );
    let second_conflict = matches!(
        &second,
        Err(TurnSessionRepositoryError::ActiveSessionConflict { .. })
    );
    assert!(
        first_conflict != second_conflict,
        "Expected exactly one conflict, got first_conflict={first_conflict}, second_conflict={second_conflict}"
    );
    if first_conflict {
        second.map_err(|err| Box::new(err) as BoxError)?;
    } else {
        first.map_err(|err| Box::new(err) as BoxError)?;
    }

    let active = ctx
        .session_repository
        .find_active_session(&ctx.ctx, SessionSlotKey::new(backend_id, conversation_id))
        .await
        .map_err(|err| Box::new(err) as BoxError)?;
    assert!(active.is_some());
    Ok(())
}

#[rstest]
#[tokio::test(flavor = "multi_thread")]
async fn postgres_session_repository_scopes_lookup_by_tenant(
    #[future] context: Result<OrchestrationContext, BoxError>,
) -> Result<(), BoxError> {
    let ctx = context.await?;
    let backend_id = corbusier::agent_backend::domain::BackendId::from_uuid(
        register_backend(&ctx, "claude_code_sdk").await?,
    );
    let conversation_id = Uuid::new_v4();
    ensure_conversation_exists(&ctx, conversation_id).await?;

    let session = TurnSession::new(TurnSessionCreateParams {
        backend_id,
        conversation_id,
        runtime_session_id: RuntimeSessionId::new("tenant-a-runtime")
            .map_err(|err| Box::new(err) as BoxError)?,
        ttl: Duration::minutes(5),
        now: Utc::now(),
    })
    .map_err(|err| Box::new(err) as BoxError)?;

    ctx.session_repository
        .upsert_session(&ctx.ctx, &session)
        .await
        .map_err(|err| Box::new(err) as BoxError)?;

    let tenant_b_ctx = other_tenant_context(&ctx.ctx);
    let tenant_b_lookup = ctx
        .session_repository
        .find_active_session(
            &tenant_b_ctx,
            SessionSlotKey::new(backend_id, conversation_id),
        )
        .await
        .map_err(|err| Box::new(err) as BoxError)?;

    assert!(tenant_b_lookup.is_none());
    Ok(())
}
