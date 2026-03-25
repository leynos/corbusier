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
    liveness_status_to_response(check.liveness())
}

async fn readiness(check: web::Data<dyn HealthCheck>) -> HttpResponse {
    readiness_status_to_response(check.readiness())
}

fn liveness_status_to_response(status: HealthStatus) -> HttpResponse {
    match status {
        HealthStatus::Healthy | HealthStatus::Degraded => HttpResponse::Ok().body("ok"),
        HealthStatus::Unhealthy => HttpResponse::ServiceUnavailable().body("unavailable"),
    }
}

fn readiness_status_to_response(status: HealthStatus) -> HttpResponse {
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
    use rstest::rstest;

    use super::*;

    struct TestHealthCheck {
        liveness: HealthStatus,
        readiness: HealthStatus,
    }

    impl HealthCheck for TestHealthCheck {
        fn liveness(&self) -> HealthStatus {
            self.liveness
        }

        fn readiness(&self) -> HealthStatus {
            self.readiness
        }
    }

    async fn assert_endpoint_status(
        liveness: HealthStatus,
        readiness: HealthStatus,
        path: &str,
        expected_status: u16,
    ) {
        let health_check: Arc<dyn HealthCheck> = Arc::new(TestHealthCheck {
            liveness,
            readiness,
        });
        let app = actix_test::init_service(
            App::new()
                .app_data(web::Data::from(health_check))
                .configure(health_routes),
        )
        .await;
        let req = actix_test::TestRequest::get().uri(path).to_request();
        let resp = actix_test::call_service(&app, req).await;
        assert_eq!(resp.status().as_u16(), expected_status);
    }

    #[rstest]
    #[case(HealthStatus::Healthy, 200)]
    #[case(HealthStatus::Degraded, 200)]
    #[case(HealthStatus::Unhealthy, 503)]
    #[actix_web::test]
    async fn liveness_routes_map_statuses(
        #[case] liveness: HealthStatus,
        #[case] expected_status: u16,
    ) {
        assert_endpoint_status(
            liveness,
            HealthStatus::Healthy,
            "/health/live",
            expected_status,
        )
        .await;
    }

    #[rstest]
    #[case(HealthStatus::Healthy, 200)]
    #[case(HealthStatus::Degraded, 503)]
    #[case(HealthStatus::Unhealthy, 503)]
    #[actix_web::test]
    async fn readiness_routes_map_statuses(
        #[case] readiness: HealthStatus,
        #[case] expected_status: u16,
    ) {
        assert_endpoint_status(
            HealthStatus::Healthy,
            readiness,
            "/health/ready",
            expected_status,
        )
        .await;
    }
}
