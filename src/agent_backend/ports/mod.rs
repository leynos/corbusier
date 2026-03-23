//! Port contracts for agent backend orchestration.
//!
//! Ports define infrastructure-agnostic interfaces for backend registration,
//! runtime execution, tool routing, and session persistence.

pub mod repository;
pub mod runtime;
pub mod session;
pub mod tool_router;

pub use repository::{BackendRegistryError, BackendRegistryRepository, BackendRegistryResult};
pub use runtime::{AgentRuntimeError, AgentRuntimePort, AgentRuntimeResult};
pub use session::{
    SessionSlotArbitration, SessionSlotKey, SessionSlotReservation, TurnSessionRepository,
    TurnSessionRepositoryError, TurnSessionRepositoryResult,
};
pub use tool_router::{ToolRouterPort, ToolRoutingContext, ToolRoutingError, ToolRoutingResult};
