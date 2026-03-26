//! Corbusier application entry point.
//!
//! Starts an HTTP server exposing health-check endpoints for
//! Kubernetes liveness and readiness probes.

use std::sync::Arc;

use actix_web::{App, HttpServer, web};
use tracing::info;

use corbusier::health::actix_adapter::health_routes;
use corbusier::health::{HealthCheck, SimpleHealthCheck};

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

    info!(port, "Starting Corbusier");

    HttpServer::new(move || {
        App::new()
            .app_data(web::Data::from(health.clone()))
            .configure(health_routes)
    })
    .bind(("0.0.0.0", port))?
    .run()
    .await
}
