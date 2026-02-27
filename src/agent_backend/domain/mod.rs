//! Domain model for agent backend registration and discovery.
//!
//! The agent backend domain models registration metadata, capability
//! declarations, and lifecycle status for pluggable AI agent backends.
//! All infrastructure concerns are kept outside the domain boundary.

mod capabilities;
mod error;
mod ids;
mod info;
mod name;
mod registration;
mod status;

pub use capabilities::AgentCapabilities;
pub use error::{BackendDomainError, ParseBackendStatusError};
pub use ids::BackendId;
pub use info::BackendInfo;
pub use name::BackendName;
pub use registration::{AgentBackendRegistration, PersistedBackendData};
pub use status::BackendStatus;
