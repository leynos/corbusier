//! Route registration for the HTTP API.

use actix_web::web;

pub mod conversations;
pub mod tasks;
pub mod tools;

/// Registers the versioned API routes.
pub fn api_routes(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/api/v1")
            .configure(conversations::routes)
            .configure(tasks::routes)
            .configure(tools::routes),
    );
}
