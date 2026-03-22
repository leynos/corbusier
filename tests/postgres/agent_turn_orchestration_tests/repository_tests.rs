//! Repository-focused orchestration persistence tests against `PostgreSQL`.

use chrono::{Duration, Utc};
use corbusier::agent_backend::{
    domain::{RuntimeSessionId, TurnSession, TurnSessionCreateParams, TurnSessionStatus},
    ports::{
        SessionSlotArbitration, SessionSlotKey, SessionSlotReservation, TurnSessionRepository,
        TurnSessionRepositoryError,
    },
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
async fn postgres_arbitration_commits_reservation_before_runtime_creation(
    #[future] context: Result<OrchestrationContext, BoxError>,
) -> Result<(), BoxError> {
    let ctx = context.await?;
    let backend_id = corbusier::agent_backend::domain::BackendId::from_uuid(
        register_backend(&ctx, "claude_code_sdk").await?,
    );
    let conversation_id = Uuid::new_v4();
    ensure_conversation_exists(&ctx, conversation_id).await?;

    let arbitration = ctx
        .session_repository
        .arbitrate_session_slot(
            &ctx.ctx,
            SessionSlotReservation::new(
                SessionSlotKey::new(backend_id, conversation_id),
                Utc::now(),
                Duration::minutes(5),
            ),
        )
        .await
        .map_err(|err| Box::new(err) as BoxError)?;

    let SessionSlotArbitration::Reserved {
        reservation,
        prior_expired,
    } = arbitration
    else {
        return Err(Box::new(std::io::Error::other(
            "expected reserved arbitration result",
        )) as BoxError);
    };
    assert!(prior_expired.is_none());

    let sessions = ctx
        .session_repository
        .all_sessions()
        .map_err(|err| Box::new(err) as BoxError)?;
    assert_eq!(sessions.len(), 1);
    let persisted = sessions
        .first()
        .ok_or_else(|| Box::new(std::io::Error::other("expected reservation row")) as BoxError)?;
    assert_eq!(persisted.id(), reservation.id());
    assert_eq!(persisted.status(), TurnSessionStatus::Reserved);
    assert!(
        ctx.runtime
            .created_session_ids()
            .map_err(|err| Box::new(err) as BoxError)?
            .is_empty()
    );
    Ok(())
}

#[rstest]
#[tokio::test(flavor = "multi_thread")]
async fn postgres_rejects_sequential_duplicate_reservation_claim(
    #[future] context: Result<OrchestrationContext, BoxError>,
) -> Result<(), BoxError> {
    let ctx = context.await?;
    let backend_id = corbusier::agent_backend::domain::BackendId::from_uuid(
        register_backend(&ctx, "claude_code_sdk").await?,
    );
    let conversation_id = Uuid::new_v4();
    ensure_conversation_exists(&ctx, conversation_id).await?;
    let reservation = SessionSlotReservation::new(
        SessionSlotKey::new(backend_id, conversation_id),
        Utc::now(),
        Duration::minutes(5),
    );

    let first = ctx
        .session_repository
        .arbitrate_session_slot(&ctx.ctx, reservation)
        .await
        .map_err(|err| Box::new(err) as BoxError)?;
    assert!(matches!(first, SessionSlotArbitration::Reserved { .. }));
    let sessions_after_first = ctx
        .session_repository
        .all_sessions()
        .map_err(|err| Box::new(err) as BoxError)?;
    assert_eq!(
        sessions_after_first.len(),
        1,
        "expected one persisted reservation, got {sessions_after_first:?}"
    );

    let second = ctx
        .session_repository
        .arbitrate_session_slot(&ctx.ctx, reservation)
        .await;
    assert!(
        matches!(
            second,
            Err(TurnSessionRepositoryError::ActiveSessionConflict { .. })
        ),
        "expected reservation conflict, got second={second:?}"
    );
    Ok(())
}

#[rstest]
#[tokio::test(flavor = "multi_thread")]
async fn postgres_detects_concurrent_reservation_conflict_before_runtime_creation(
    #[future] context: Result<OrchestrationContext, BoxError>,
) -> Result<(), BoxError> {
    let ctx = context.await?;
    let backend_id = corbusier::agent_backend::domain::BackendId::from_uuid(
        register_backend(&ctx, "claude_code_sdk").await?,
    );
    let conversation_id = Uuid::new_v4();
    ensure_conversation_exists(&ctx, conversation_id).await?;

    let now = Utc::now();
    let key = SessionSlotKey::new(backend_id, conversation_id);
    let (first, second) = tokio::join!(
        ctx.session_repository.arbitrate_session_slot(
            &ctx.ctx,
            SessionSlotReservation::new(key, now, Duration::minutes(5)),
        ),
        ctx.session_repository.arbitrate_session_slot(
            &ctx.ctx,
            SessionSlotReservation::new(key, now, Duration::minutes(5)),
        )
    );

    let first_reserved = matches!(&first, Ok(SessionSlotArbitration::Reserved { .. }));
    let second_reserved = matches!(&second, Ok(SessionSlotArbitration::Reserved { .. }));
    let first_conflict = matches!(
        &first,
        Err(TurnSessionRepositoryError::ActiveSessionConflict { .. })
    );
    let second_conflict = matches!(
        &second,
        Err(TurnSessionRepositoryError::ActiveSessionConflict { .. })
    );
    assert!(
        first_reserved != second_reserved,
        "expected exactly one reserved result, got first={first:?}, second={second:?}"
    );
    assert!(
        first_conflict != second_conflict,
        "expected exactly one reservation conflict, got first={first:?}, second={second:?}"
    );

    let sessions = ctx
        .session_repository
        .all_sessions()
        .map_err(|err| Box::new(err) as BoxError)?;
    assert_eq!(sessions.len(), 1);
    let persisted = sessions
        .first()
        .ok_or_else(|| Box::new(std::io::Error::other("expected reservation row")) as BoxError)?;
    assert_eq!(persisted.status(), TurnSessionStatus::Reserved);
    assert!(
        ctx.runtime
            .created_session_ids()
            .map_err(|err| Box::new(err) as BoxError)?
            .is_empty()
    );
    Ok(())
}

#[rstest]
#[tokio::test(flavor = "multi_thread")]
async fn postgres_reclaims_timed_out_reservation_after_crash(
    #[future] context: Result<OrchestrationContext, BoxError>,
) -> Result<(), BoxError> {
    let ctx = context.await?;
    let backend_id = corbusier::agent_backend::domain::BackendId::from_uuid(
        register_backend(&ctx, "claude_code_sdk").await?,
    );
    let conversation_id = Uuid::new_v4();
    ensure_conversation_exists(&ctx, conversation_id).await?;

    let initial = ctx
        .session_repository
        .arbitrate_session_slot(
            &ctx.ctx,
            SessionSlotReservation::new(
                SessionSlotKey::new(backend_id, conversation_id),
                Utc::now(),
                Duration::seconds(1),
            ),
        )
        .await
        .map_err(|err| Box::new(err) as BoxError)?;
    let SessionSlotArbitration::Reserved {
        reservation: initial_reservation,
        ..
    } = initial
    else {
        return Err(Box::new(std::io::Error::other("expected initial reservation")) as BoxError);
    };

    let reclaimed = ctx
        .session_repository
        .arbitrate_session_slot(
            &ctx.ctx,
            SessionSlotReservation::new(
                SessionSlotKey::new(backend_id, conversation_id),
                Utc::now() + Duration::seconds(2),
                Duration::minutes(5),
            ),
        )
        .await
        .map_err(|err| Box::new(err) as BoxError)?;
    let SessionSlotArbitration::Reserved {
        reservation,
        prior_expired,
    } = reclaimed
    else {
        return Err(Box::new(std::io::Error::other("expected reclaimed reservation")) as BoxError);
    };
    // Reclaiming a timed-out `Reserved` row intentionally keeps
    // `prior_expired` empty because only previously active sessions need
    // runtime teardown after `SessionSlotArbitration::Reserved`.
    assert!(prior_expired.is_none());
    assert_ne!(reservation.id(), initial_reservation.id());

    let sessions = ctx
        .session_repository
        .all_sessions()
        .map_err(|err| Box::new(err) as BoxError)?;
    let expired = sessions
        .iter()
        .find(|session| session.id() == initial_reservation.id())
        .ok_or_else(|| {
            Box::new(std::io::Error::other("expected expired reservation row")) as BoxError
        })?;
    assert_eq!(expired.status(), TurnSessionStatus::Expired);
    let active_claims = sessions
        .iter()
        .filter(|session| session.status() == TurnSessionStatus::Reserved)
        .count();
    assert_eq!(active_claims, 1);
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
