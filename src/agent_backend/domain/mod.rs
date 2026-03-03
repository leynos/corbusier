//! Domain model for agent backend registration, turn execution, and sessions.
//!
//! The agent backend domain models registration metadata, turn execution value
//! objects, and session lifecycle state for pluggable AI agent backends. All
//! infrastructure concerns are kept outside the domain boundary.

mod capabilities;
mod error;
mod ids;
mod info;
mod name;
mod registration;
mod session;
mod status;
mod turn;

pub use capabilities::AgentCapabilities;
pub use error::{BackendDomainError, ParseBackendStatusError};
pub use ids::BackendId;
pub use info::BackendInfo;
pub use name::BackendName;
pub use registration::{AgentBackendRegistration, PersistedBackendData};
pub use session::{
    ParseTurnSessionStatusError, PersistedTurnSessionData, RuntimeSessionId, TurnSession,
    TurnSessionCreateParams, TurnSessionDomainError, TurnSessionId, TurnSessionStatus,
};
pub use status::BackendStatus;
pub use turn::{
    ToolCallAudit, ToolCallAuditStatus, ToolCallRequest, ToolCallResult, TurnDomainError,
    TurnExecutionRequest, TurnExecutionResult, deterministic_tool_call_id,
};
