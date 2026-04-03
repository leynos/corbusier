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
use std::fmt;
use std::sync::Arc;
use uuid::Uuid;

type PostgresToolService = ToolDiscoveryRoutingService<
    PostgresToolCatalog,
    PostgresMcpServerRegistry,
    InMemoryMcpServerHost,
    AllowAllPolicy,
    ObjectStoreLogAdapter,
    DefaultClock,
>;

type PostgresLifecycleService =
    McpServerLifecycleService<PostgresMcpServerRegistry, InMemoryMcpServerHost, DefaultClock>;

type ToolDependencies = (
    Arc<PostgresMcpServerRegistry>,
    Arc<PostgresToolCatalog>,
    Arc<InMemoryMcpServerHost>,
    Arc<PostgresLifecycleService>,
    Arc<PostgresToolService>,
);

#[derive(Debug)]
struct BootstrapError {
    source: eyre::Report,
}

impl fmt::Display for BootstrapError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "file_tools bootstrap failed")
    }
}

impl std::error::Error for BootstrapError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        Some(self.source.as_ref())
    }
}

/// Use this static JWT secret for `PostgreSQL` HTTP API integration tests.
///
/// Keep it aligned with the test-only authenticator and token helper.
pub(crate) const TEST_JWT_SECRET: &str = "test-http-api-secret";

/// Hold the shared state and auth helper for `PostgreSQL` HTTP API tests.
///
/// Keep `_temp_db` alive for the full test lifetime so teardown can drop the
/// temporary database after the test finishes.
pub(crate) struct PostgresHttpApiContext {
    pub(crate) state: ApiState,
    pub(crate) auth: HttpApiAuth,
    _temp_db: TemporaryDatabase,
}

/// Build the `PostgreSQL` connection pool for a temporary test database.
pub(crate) fn build_pool(db: &TemporaryDatabase) -> Result<PgPool, BoxError> {
    let manager = ConnectionManager::<PgConnection>::new(db.url());
    diesel::r2d2::Pool::builder()
        .max_size(2)
        .build(manager)
        .map_err(|err| Box::new(err) as BoxError)
}

/// Build the API state and auth helper for a `PostgreSQL` HTTP API test run.
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

fn assemble_tool_dependencies(pool: PgPool, clock: Arc<DefaultClock>) -> ToolDependencies {
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
            catalog: catalog.clone(),
            registry: registry.clone(),
            host: host.clone(),
            governance: Arc::new(AllowAllPolicy::new()),
            log_store: Arc::new(ObjectStoreLogAdapter::in_memory()),
        },
        LogRetentionPolicy::default(),
        clock,
    ));

    (registry, catalog, host, lifecycle, tool_service)
}

async fn wire_bootstrap(
    host: &InMemoryMcpServerHost,
    lifecycle: Arc<PostgresLifecycleService>,
    tool_service: Arc<PostgresToolService>,
    ctx: &corbusier::context::RequestContext,
) -> Result<(), BoxError> {
    let register_service = lifecycle.clone();
    let start_service = lifecycle;
    let discovery_service = tool_service;
    let register_request_ctx = ctx.clone();
    let start_request_ctx = ctx.clone();
    let discover_request_ctx = ctx.clone();

    bootstrap_file_tools_server(
        host,
        move |request| {
            let register_service_clone = register_service.clone();
            let register_request_ctx_clone = register_request_ctx.clone();
            async move {
                register_service_clone
                    .as_ref()
                    .register(&register_request_ctx_clone, request)
                    .await
                    .map_err(eyre::Report::from)
            }
        },
        move |server_id| {
            let start_service_clone = start_service.clone();
            let start_request_ctx_clone = start_request_ctx.clone();
            async move {
                start_service_clone
                    .as_ref()
                    .start(&start_request_ctx_clone, server_id)
                    .await
                    .map(|_| ())
                    .map_err(eyre::Report::from)
            }
        },
        move |server_id| {
            let discovery_service_clone = discovery_service.clone();
            let discover_request_ctx_clone = discover_request_ctx.clone();
            async move {
                discovery_service_clone
                    .discover_and_persist_tools(&discover_request_ctx_clone, server_id)
                    .await
                    .map(|_| ())
                    .map_err(eyre::Report::from)
            }
        },
    )
    .await
    .map_err(|source| Box::new(BootstrapError { source }) as BoxError)
}

/// Build the shared tool service and bootstrap the in-memory MCP host.
pub(crate) async fn build_tool_service(
    pool: PgPool,
    ctx: &corbusier::context::RequestContext,
    clock: Arc<DefaultClock>,
) -> Result<Arc<PostgresToolService>, BoxError> {
    let (_registry, _catalog, host, lifecycle, tool_service) =
        assemble_tool_dependencies(pool, clock);
    wire_bootstrap(host.as_ref(), lifecycle, tool_service.clone(), ctx).await?;

    Ok(tool_service)
}

/// Create the full PostgreSQL-backed HTTP API test context.
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

/// Build the `PostgreSQL` HTTP API fixture after ensuring the template database
/// exists.
#[fixture]
pub(crate) async fn context(
    postgres_cluster: Result<PostgresCluster, BoxError>,
) -> Result<PostgresHttpApiContext, BoxError> {
    let cluster = postgres_cluster?;
    ensure_template(cluster).await?;
    setup_context(cluster).await
}
