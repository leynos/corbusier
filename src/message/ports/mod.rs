//! Port trait definitions for the message subsystem.
//!
//! Ports define the abstract interfaces that the domain requires from
//! infrastructure. Adapters implement these ports to connect the domain
//! to databases, external services, and other infrastructure.

pub mod agent_session;
pub mod context_snapshot;
pub mod handoff;
pub mod repository;
pub mod validator;

pub use agent_session::{AgentSessionRepository, SessionError, SessionResult};
pub use context_snapshot::{ContextSnapshotPort, SnapshotError, SnapshotResult};
pub use handoff::{AgentHandoffPort, HandoffError, HandoffResult};
pub use repository::MessageRepository;
pub use validator::{MessageValidator, ValidationConfig};
