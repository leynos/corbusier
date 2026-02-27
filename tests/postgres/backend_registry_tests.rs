//! `PostgreSQL` integration tests for agent backend registration and discovery.

use corbusier::agent_backend::{
    adapters::postgres::{BackendPgPool, PostgresBackendRegistry},
    domain::BackendStatus,
    ports::BackendRegistryError,
    services::{BackendRegistryService, BackendRegistryServiceError, RegisterBackendRequest},
};
use diesel::PgConnection;
use diesel::r2d2::ConnectionManager;
use mockable::DefaultClock;
use rstest::{fixture, rstest};
use std::sync::Arc;
use uuid::Uuid;

use crate::postgres::cluster::TemporaryDatabase;
use crate::postgres::helpers::{
    BoxError, PostgresCluster, TEMPLATE_DB, ensure_template, postgres_cluster,
};

type TestService = BackendRegistryService<PostgresBackendRegistry, DefaultClock>;

struct BackendTestContext {
    service: TestService,
    _temp_db: TemporaryDatabase,
}

async fn setup_backend_context(cluster: PostgresCluster) -> Result<BackendTestContext, BoxError> {
    let db = cluster
        .temporary_database_from_template(&format!("backend_{}", Uuid::new_v4()), TEMPLATE_DB)
        .await?;
    let url = db.url().to_owned();

    let manager = ConnectionManager::<PgConnection>::new(url);
    let pool: BackendPgPool = diesel::r2d2::Pool::builder()
        .max_size(1)
        .build(manager)
        .map_err(|err| Box::new(err) as BoxError)?;
    let repository = Arc::new(PostgresBackendRegistry::new(pool));
    let service = BackendRegistryService::new(repository, Arc::new(DefaultClock));
    Ok(BackendTestContext {
        service,
        _temp_db: db,
    })
}

#[fixture]
async fn context(
    postgres_cluster: Result<PostgresCluster, BoxError>,
) -> Result<BackendTestContext, BoxError> {
    let cluster = postgres_cluster?;
    ensure_template(cluster).await?;
    setup_backend_context(cluster).await
}

fn claude_request() -> RegisterBackendRequest {
    RegisterBackendRequest::new("claude_code_sdk", "Claude Code SDK", "1.0.0", "Anthropic")
        .with_capabilities(true, true)
}

fn codex_request() -> RegisterBackendRequest {
    RegisterBackendRequest::new("codex_cli", "Codex CLI", "0.9.0", "OpenAI")
        .with_capabilities(false, true)
}

#[rstest]
#[tokio::test(flavor = "multi_thread")]
async fn postgres_register_and_retrieve_by_id(
    #[future] context: Result<BackendTestContext, BoxError>,
) -> Result<(), BoxError> {
    let ctx = context.await?;
    let created = ctx
        .service
        .register(claude_request())
        .await
        .expect("registration should succeed");

    let found = ctx
        .service
        .find_by_id(created.id())
        .await
        .expect("lookup should succeed")
        .expect("backend should exist");

    assert_eq!(found.id(), created.id());
    assert_eq!(found.name(), created.name());
    assert_eq!(found.status(), created.status());
    assert_eq!(found.backend_info(), created.backend_info());
    assert_eq!(found.capabilities(), created.capabilities());
    Ok(())
}

#[rstest]
#[tokio::test(flavor = "multi_thread")]
async fn postgres_register_and_retrieve_by_name(
    #[future] context: Result<BackendTestContext, BoxError>,
) -> Result<(), BoxError> {
    let ctx = context.await?;
    let created = ctx
        .service
        .register(claude_request())
        .await
        .expect("registration should succeed");

    let found = ctx
        .service
        .find_by_name("claude_code_sdk")
        .await
        .expect("lookup should succeed")
        .expect("backend should exist");

    assert_eq!(found.id(), created.id());
    assert_eq!(found.name(), created.name());
    Ok(())
}

#[rstest]
#[tokio::test(flavor = "multi_thread")]
async fn postgres_duplicate_name_is_rejected(
    #[future] context: Result<BackendTestContext, BoxError>,
) -> Result<(), BoxError> {
    let ctx = context.await?;
    ctx.service
        .register(claude_request())
        .await
        .expect("first registration should succeed");

    let result = ctx.service.register(claude_request()).await;

    assert!(matches!(
        result,
        Err(BackendRegistryServiceError::Repository(
            BackendRegistryError::DuplicateBackendName(_)
        ))
    ));
    Ok(())
}

#[rstest]
#[tokio::test(flavor = "multi_thread")]
async fn postgres_list_active_excludes_inactive(
    #[future] context: Result<BackendTestContext, BoxError>,
) -> Result<(), BoxError> {
    let ctx = context.await?;
    let claude = ctx
        .service
        .register(claude_request())
        .await
        .expect("first registration should succeed");
    ctx.service
        .register(codex_request())
        .await
        .expect("second registration should succeed");

    ctx.service
        .deactivate(claude.id())
        .await
        .expect("deactivation should succeed");

    let active = ctx
        .service
        .list_active()
        .await
        .expect("listing should succeed");
    assert_eq!(active.len(), 1);
    let first = active.first().expect("one entry");
    assert_eq!(first.name().as_str(), "codex_cli");
    assert_eq!(first.status(), BackendStatus::Active);
    Ok(())
}

#[rstest]
#[tokio::test(flavor = "multi_thread")]
async fn postgres_list_all_includes_inactive(
    #[future] context: Result<BackendTestContext, BoxError>,
) -> Result<(), BoxError> {
    let ctx = context.await?;
    let claude = ctx
        .service
        .register(claude_request())
        .await
        .expect("first registration should succeed");
    ctx.service
        .register(codex_request())
        .await
        .expect("second registration should succeed");

    ctx.service
        .deactivate(claude.id())
        .await
        .expect("deactivation should succeed");

    let all = ctx
        .service
        .list_all()
        .await
        .expect("listing should succeed");
    assert_eq!(all.len(), 2);
    Ok(())
}
