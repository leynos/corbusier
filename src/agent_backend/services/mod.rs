//! Application services for agent backend registration and discovery.

mod registry;

pub use registry::{BackendRegistryService, BackendRegistryServiceError, RegisterBackendRequest};
