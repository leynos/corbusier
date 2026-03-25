//! Health observation port and default implementation.
//!
//! This module defines the domain vocabulary for application health
//! (`HealthStatus`) and a port trait (`HealthCheck`) that adapters
//! (HTTP, gRPC, CLI) call to obtain liveness and readiness status.
//!
//! The `SimpleHealthCheck` implementation always reports `Healthy`,
//! suitable for the initial deployment. Future milestones can inject
//! real dependency probes (Postgres connectivity, Valkey ping) via
//! the same port.

pub mod actix_adapter;

/// Health status reported by the application.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HealthStatus {
    /// The application is fully operational.
    Healthy,
    /// The application is operational but degraded.
    Degraded,
    /// The application is not operational.
    Unhealthy,
}

/// Port for health observation.
///
/// Adapters (HTTP, gRPC, CLI) call this to obtain the application's
/// liveness and readiness status.
pub trait HealthCheck: Send + Sync {
    /// Report whether the process is alive.
    fn liveness(&self) -> HealthStatus;
    /// Report whether the process is ready to serve traffic.
    fn readiness(&self) -> HealthStatus;
}

/// Simple health check that always reports [`HealthStatus::Healthy`].
///
/// # Examples
///
/// ```
/// use corbusier::health::{HealthCheck, HealthStatus, SimpleHealthCheck};
///
/// let check = SimpleHealthCheck;
/// assert_eq!(check.liveness(), HealthStatus::Healthy);
/// assert_eq!(check.readiness(), HealthStatus::Healthy);
/// ```
pub struct SimpleHealthCheck;

impl HealthCheck for SimpleHealthCheck {
    fn liveness(&self) -> HealthStatus {
        HealthStatus::Healthy
    }
    fn readiness(&self) -> HealthStatus {
        HealthStatus::Healthy
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn simple_health_check_reports_healthy() {
        let check = SimpleHealthCheck;
        assert_eq!(check.liveness(), HealthStatus::Healthy);
        assert_eq!(check.readiness(), HealthStatus::Healthy);
    }
}
