//! Shared `PostgreSQL` HTTP API integration-test setup.

use crate::http_api_test_helpers::{HttpApiAuth, bootstrap_file_tools_server};
use crate::postgres::cluster::TemporaryDatabase;
use crate::postgres::helpers::{
    BoxError, PostgresCluster, TEMPLATE_DB, ensure_template, postgres_cluster,
};
use corbusier::{
    http_api::{ApiConfig, ApiState, BearerTokenAuthenticator},
    message::{
        adapters::postgres::{PgPool, PostgresConversationRepository, PostgresMessageRepository},
        services::ConversationService,
        validation::service::DefaultMessageValidator,
    },
    task::{adapters::postgres::PostgresTaskRepository, services::TaskLifecycleService},
    tool_registry::{
        adapters::{
            AllowAllPolicy, InMemoryMcpServerHost, ObjectStoreLogAdapter,
            postgres::{PostgresMcpServerRegistry, PostgresToolCatalog},
        },
        domain::LogRetentionPolicy,
        services::{McpServerLifecycleService, ServicePorts, ToolDiscoveryRoutingService},
    },
};
use diesel::{PgConnection, r2d2::ConnectionManager};
use mockable::{Clock, DefaultClock};
use rstest::fixture;
use std::sync::Arc;
use uuid::Uuid;

pub(crate) const TEST_JWT_SECRET: &str = "test-http-api-secret";

type PostgresToolService = ToolDiscoveryRoutingService<
    PostgresToolCatalog,
    PostgresMcpServerRegistry,
    InMemoryMcpServerHost,
    AllowAllPolicy,
    ObjectStoreLogAdapter,
    DefaultClock,
>;

pub(crate) struct PostgresHttpApiContext {
    pub(crate) state: ApiState,
    pub(crate) auth: HttpApiAuth,
    _temp_db: TemporaryDatabase,
}

pub(crate) fn build_pool(db: &TemporaryDatabase) -> Result<PgPool, BoxError> {
    let manager = ConnectionManager::<PgConnection>::new(db.url());
    diesel::r2d2::Pool::builder()
        .max_size(2)
        .build(manager)
        .map_err(|err| Box::new(err) as BoxError)
}

pub(crate) async fn build_state(pool: PgPool) -> Result<(ApiState, HttpApiAuth), BoxError> {
    let auth = HttpApiAuth::new(TEST_JWT_SECRET);
    let ctx = auth.request_context();
    let clock = Arc::new(DefaultClock);

    let conversation_service = Arc::new(ConversationService::new(
        Arc::new(PostgresConversationRepository::new(pool.clone())),
        Arc::new(PostgresMessageRepository::new(pool.clone())),
        Arc::new(DefaultMessageValidator::new()),
        clock.clone(),
    ));
    let task_service = Arc::new(TaskLifecycleService::new(
        Arc::new(PostgresTaskRepository::new(pool.clone())),
        clock.clone(),
    ));
    let tool_service = build_tool_service(pool, &ctx, clock.clone()).await?;

    Ok((
        ApiState::new(
            conversation_service,
            task_service,
            tool_service,
            ApiConfig {
                authenticator: BearerTokenAuthenticator::new(TEST_JWT_SECRET),
                clock: clock as Arc<dyn Clock + Send + Sync>,
            },
        ),
        auth,
    ))
}

pub(crate) async fn build_tool_service(
    pool: PgPool,
    ctx: &corbusier::context::RequestContext,
    clock: Arc<DefaultClock>,
) -> Result<Arc<PostgresToolService>, BoxError> {
    let registry = Arc::new(PostgresMcpServerRegistry::new(pool.clone()));
    let catalog = Arc::new(PostgresToolCatalog::new(pool));
    let host = Arc::new(InMemoryMcpServerHost::new());
    let lifecycle = Arc::new(McpServerLifecycleService::new(
        registry.clone(),
        host.clone(),
        clock.clone(),
    ));
    let tool_service = Arc::new(ToolDiscoveryRoutingService::new(
        ServicePorts {
            catalog,
            registry: registry.clone(),
            host: host.clone(),
            governance: Arc::new(AllowAllPolicy::new()),
            log_store: Arc::new(ObjectStoreLogAdapter::in_memory()),
        },
        LogRetentionPolicy::default(),
        clock,
    ));
    let register_lifecycle = lifecycle.clone();
    let start_lifecycle = lifecycle;
    let discover_service = tool_service.clone();

    bootstrap_file_tools_server(
        host.as_ref(),
        |request| async move {
            register_lifecycle
                .as_ref()
                .register(ctx, request)
                .await
                .map_err(eyre::Report::from)
        },
        |server_id| async move {
            start_lifecycle
                .as_ref()
                .start(ctx, server_id)
                .await
                .map(|_| ())
                .map_err(eyre::Report::from)
        },
        |server_id| async move {
            discover_service
                .discover_and_persist_tools(ctx, server_id)
                .await
                .map(|_| ())
                .map_err(eyre::Report::from)
        },
    )
    .await
    .map_err(|err| Box::new(std::io::Error::other(err.to_string())) as BoxError)?;

    Ok(tool_service)
}

pub(crate) async fn setup_context(
    cluster: PostgresCluster,
) -> Result<PostgresHttpApiContext, BoxError> {
    let db = cluster
        .temporary_database_from_template(&format!("http_api_{}", Uuid::new_v4()), TEMPLATE_DB)
        .await?;
    let pool = build_pool(&db)?;
    let (state, auth) = build_state(pool).await?;

    Ok(PostgresHttpApiContext {
        state,
        auth,
        _temp_db: db,
    })
}

#[fixture]
pub(crate) async fn context(
    postgres_cluster: Result<PostgresCluster, BoxError>,
) -> Result<PostgresHttpApiContext, BoxError> {
    let cluster = postgres_cluster?;
    ensure_template(cluster).await?;
    setup_context(cluster).await
}
