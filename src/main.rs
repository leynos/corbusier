//! Corbusier application entry point.
//!
//! Starts an HTTP server exposing health-check and core API routes.

use std::sync::Arc;

use actix_web::{App, HttpServer, web};
use corbusier::{
    health::{HealthCheck, SimpleHealthCheck, actix_adapter::health_routes},
    http_api::{
        ApiConfig, ApiState, BearerTokenAuthenticator, api_routes, auth::request_correlation_id,
        error::ApiError,
    },
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
        services::{ServicePorts, ToolDiscoveryRoutingService},
    },
};
use diesel::{
    PgConnection,
    r2d2::{ConnectionManager, Pool},
};
use mockable::{Clock, DefaultClock};
use tracing::info;

/// Default HTTP listen port.
const DEFAULT_PORT: u16 = 8080;

/// Application entry point.
///
/// Starts an Actix Web server on the port specified by the
/// `CORBUSIER_PORT` environment variable (default 8080).
#[actix_web::main]
async fn main() -> std::io::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .init();

    let port = std::env::var("CORBUSIER_PORT")
        .ok()
        .and_then(|v| v.parse::<u16>().ok())
        .unwrap_or(DEFAULT_PORT);

    let health: Arc<dyn HealthCheck> = Arc::new(SimpleHealthCheck);
    let api_state = web::Data::new(build_api_state()?);
    let api_clock = api_state.clock.clone();

    info!(port, "Starting Corbusier");

    HttpServer::new(move || {
        let json_clock = api_clock.clone();
        App::new()
            .app_data(web::Data::from(health.clone()))
            .app_data(api_state.clone())
            .app_data(
                web::JsonConfig::default().error_handler(move |error, request| {
                    let error_message = error.to_string();
                    let request_id = request_correlation_id(request).to_string();
                    actix_web::error::InternalError::from_response(
                        error,
                        ApiError::bad_request("invalid_json", error_message)
                            .into_response(&*json_clock, request_id),
                    )
                    .into()
                }),
            )
            .configure(health_routes)
            .configure(api_routes)
    })
    .bind(("0.0.0.0", port))?
    .run()
    .await
}

fn build_api_state() -> std::io::Result<ApiState> {
    let database_url = required_env("DATABASE_URL")?;
    let jwt_secret = required_env("CORBUSIER_JWT_SECRET")?;
    let pool = build_pg_pool(&database_url)?;
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

    // TODO: Replace InMemoryMcpServerHost with a persistent adapter (e.g.,
    // PostgresMcpServerHost) for production horizontal scalability.
    let tool_service = Arc::new(ToolDiscoveryRoutingService::new(
        ServicePorts {
            catalog: Arc::new(PostgresToolCatalog::new(pool.clone())),
            registry: Arc::new(PostgresMcpServerRegistry::new(pool)),
            // FIXME: InMemoryMcpServerHost is not persisted across restarts.
            // Replace with a persistent host implementation before production.
            host: Arc::new(InMemoryMcpServerHost::new()),
            // FIXME: AllowAllPolicy bypasses governance enforcement.
            // Replace with a production governance policy before release.
            governance: Arc::new(AllowAllPolicy::new()),
            // FIXME: In-memory log storage does not persist audit logs.
            // Replace with a production log store before release.
            log_store: Arc::new(ObjectStoreLogAdapter::in_memory()),
        },
        LogRetentionPolicy::default(),
        clock.clone(),
    ));

    Ok(ApiState::new(
        conversation_service,
        task_service,
        tool_service,
        ApiConfig {
            authenticator: BearerTokenAuthenticator::new(jwt_secret),
            clock: clock as Arc<dyn Clock + Send + Sync>,
        },
    ))
}

fn required_env(name: &str) -> std::io::Result<String> {
    let value = std::env::var(name).map_err(|_| {
        std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            format!("missing required environment variable: {name}"),
        )
    })?;
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            format!("required environment variable is empty: {name}"),
        ));
    }
    Ok(trimmed.to_owned())
}

fn build_pg_pool(database_url: &str) -> std::io::Result<PgPool> {
    let manager = ConnectionManager::<PgConnection>::new(database_url);
    Pool::builder()
        .max_size(20)
        .min_idle(Some(5))
        .connection_timeout(std::time::Duration::from_secs(30))
        .build(manager)
        .map_err(|error| {
            std::io::Error::other(format!("failed to create PostgreSQL pool: {error}"))
        })
}
