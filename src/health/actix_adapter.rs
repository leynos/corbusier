//! Actix Web adapter for the [`HealthCheck`] port.
//!
//! Maps liveness and readiness queries to HTTP `GET` endpoints at
//! `/health/live` and `/health/ready`. The adapter delegates to the
//! port trait and translates [`HealthStatus`] to HTTP status codes.

use actix_web::{HttpResponse, web};

use super::{HealthCheck, HealthStatus};

/// Register health-check routes on an Actix Web service config.
///
/// # Examples
///
/// ```rust,no_run
/// use std::sync::Arc;
/// use actix_web::{web, App, HttpServer};
/// use corbusier::health::{SimpleHealthCheck, HealthCheck};
/// use corbusier::health::actix_adapter::health_routes;
///
/// let check: Arc<dyn HealthCheck> = Arc::new(SimpleHealthCheck);
/// HttpServer::new(move || {
///     App::new()
///         .app_data(web::Data::from(check.clone()))
///         .configure(health_routes)
/// });
/// ```
pub fn health_routes(cfg: &mut web::ServiceConfig) {
    cfg.route("/health/live", web::get().to(liveness))
        .route("/health/ready", web::get().to(readiness));
}

async fn liveness(check: web::Data<dyn HealthCheck>) -> HttpResponse {
    status_to_response(check.liveness())
}

async fn readiness(check: web::Data<dyn HealthCheck>) -> HttpResponse {
    status_to_response(check.readiness())
}

fn status_to_response(status: HealthStatus) -> HttpResponse {
    match status {
        HealthStatus::Healthy => HttpResponse::Ok().body("ok"),
        HealthStatus::Degraded | HealthStatus::Unhealthy => {
            HttpResponse::ServiceUnavailable().body("unavailable")
        }
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use actix_web::App;
    use actix_web::test as actix_test;
    use rstest::{fixture, rstest};

    use super::*;
    use crate::health::SimpleHealthCheck;

    #[fixture]
    fn health_check() -> Arc<dyn HealthCheck> {
        Arc::new(SimpleHealthCheck)
    }

    #[rstest]
    #[case("/health/live")]
    #[case("/health/ready")]
    #[actix_web::test]
    async fn health_routes_return_ok(health_check: Arc<dyn HealthCheck>, #[case] path: &str) {
        let app = actix_test::init_service(
            App::new()
                .app_data(web::Data::from(health_check))
                .configure(health_routes),
        )
        .await;

        let req = actix_test::TestRequest::get().uri(path).to_request();
        let resp = actix_test::call_service(&app, req).await;
        assert_eq!(resp.status(), 200);
    }
}
