//! `PostgreSQL` integration tests for agent backend registration and discovery.

use corbusier::agent_backend::{
    adapters::postgres::{BackendPgPool, PostgresBackendRegistry},
    domain::BackendStatus,
    ports::BackendRegistryError,
    services::{BackendRegistryService, BackendRegistryServiceError, RegisterBackendRequest},
};
use corbusier::context::RequestContext;
use diesel::PgConnection;
use diesel::r2d2::ConnectionManager;
use mockable::DefaultClock;
use rstest::{fixture, rstest};
use std::sync::Arc;
use uuid::Uuid;

use crate::postgres::cluster::TemporaryDatabase;
use crate::postgres::helpers::{
    BoxError, PostgresCluster, TEMPLATE_DB, ensure_template, other_tenant_ctx, postgres_cluster,
    test_request_context,
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
    test_request_context: RequestContext,
) -> Result<(), BoxError> {
    let bctx = context.await?;
    let req_ctx = test_request_context;
    let created = bctx
        .service
        .register(&req_ctx, claude_request())
        .await
        .expect("registration should succeed");

    let found = bctx
        .service
        .find_by_id(&req_ctx, created.id())
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
    test_request_context: RequestContext,
) -> Result<(), BoxError> {
    let bctx = context.await?;
    let req_ctx = test_request_context;
    let created = bctx
        .service
        .register(&req_ctx, claude_request())
        .await
        .expect("registration should succeed");

    let found = bctx
        .service
        .find_by_name(&req_ctx, "claude_code_sdk")
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
    test_request_context: RequestContext,
) -> Result<(), BoxError> {
    let bctx = context.await?;
    let req_ctx = test_request_context;
    bctx.service
        .register(&req_ctx, claude_request())
        .await
        .expect("first registration should succeed");

    let result = bctx.service.register(&req_ctx, claude_request()).await;

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
    test_request_context: RequestContext,
) -> Result<(), BoxError> {
    let bctx = context.await?;
    let req_ctx = test_request_context;
    let claude = bctx
        .service
        .register(&req_ctx, claude_request())
        .await
        .expect("first registration should succeed");
    bctx.service
        .register(&req_ctx, codex_request())
        .await
        .expect("second registration should succeed");

    bctx.service
        .deactivate(&req_ctx, claude.id())
        .await
        .expect("deactivation should succeed");

    let active = bctx
        .service
        .list_active(&req_ctx)
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
    test_request_context: RequestContext,
) -> Result<(), BoxError> {
    let bctx = context.await?;
    let req_ctx = test_request_context;
    let claude = bctx
        .service
        .register(&req_ctx, claude_request())
        .await
        .expect("first registration should succeed");
    bctx.service
        .register(&req_ctx, codex_request())
        .await
        .expect("second registration should succeed");

    bctx.service
        .deactivate(&req_ctx, claude.id())
        .await
        .expect("deactivation should succeed");

    let all = bctx
        .service
        .list_all(&req_ctx)
        .await
        .expect("listing should succeed");
    assert_eq!(all.len(), 2);
    Ok(())
}

#[rstest]
#[tokio::test(flavor = "multi_thread")]
async fn postgres_scopes_backends_per_tenant(
    #[future] context: Result<BackendTestContext, BoxError>,
    test_request_context: RequestContext,
) -> Result<(), BoxError> {
    let bctx = context.await?;
    let tenant_a_ctx = test_request_context;
    let tenant_b_ctx = other_tenant_ctx(&tenant_a_ctx);

    let tenant_a_backend = bctx
        .service
        .register(&tenant_a_ctx, claude_request())
        .await
        .expect("tenant A registration should succeed");

    let tenant_b_backend = bctx
        .service
        .register(&tenant_b_ctx, claude_request())
        .await
        .expect("tenant B registration should succeed with same name");

    assert_ne!(tenant_a_backend.id(), tenant_b_backend.id());

    let lookup_from_b = bctx
        .service
        .find_by_id(&tenant_b_ctx, tenant_a_backend.id())
        .await
        .expect("tenant-scoped lookup should succeed");
    assert!(lookup_from_b.is_none());

    let list_a = bctx
        .service
        .list_all(&tenant_a_ctx)
        .await
        .expect("tenant A list should succeed");
    let list_b = bctx
        .service
        .list_all(&tenant_b_ctx)
        .await
        .expect("tenant B list should succeed");

    assert_eq!(list_a.len(), 1);
    assert_eq!(list_b.len(), 1);
    assert_eq!(
        list_a.first().expect("tenant A backend").id(),
        tenant_a_backend.id()
    );
    assert_eq!(
        list_b.first().expect("tenant B backend").id(),
        tenant_b_backend.id()
    );
    Ok(())
}
#[rstest]
#[tokio::test(flavor = "multi_thread")]
async fn postgres_duplicate_name_is_scoped_per_tenant(
    #[future] context: Result<BackendTestContext, BoxError>,
    test_request_context: RequestContext,
) -> Result<(), BoxError> {
    let bctx = context.await?;
    let tenant_a = test_request_context;
    let tenant_b = other_tenant_ctx(&tenant_a);

    let backend_a = bctx
        .service
        .register(&tenant_a, claude_request())
        .await
        .expect("tenant A registration should succeed");
    let backend_b = bctx
        .service
        .register(&tenant_b, claude_request())
        .await
        .expect("tenant B registration should succeed");

    assert_ne!(
        backend_a.id(),
        backend_b.id(),
        "tenants must get distinct rows"
    );

    let found_a = bctx
        .service
        .find_by_name(&tenant_a, "claude_code_sdk")
        .await
        .expect("tenant A lookup should succeed")
        .expect("tenant A backend should exist");
    let found_b = bctx
        .service
        .find_by_name(&tenant_b, "claude_code_sdk")
        .await
        .expect("tenant B lookup should succeed")
        .expect("tenant B backend should exist");

    assert_eq!(found_a.id(), backend_a.id());
    assert_eq!(found_b.id(), backend_b.id());
    assert_eq!(bctx.service.list_all(&tenant_a).await?.len(), 1);
    assert_eq!(bctx.service.list_all(&tenant_b).await?.len(), 1);
    Ok(())
}
