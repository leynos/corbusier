//! Diesel model types for message persistence.
//!
//! These types map database rows to Rust structs using Diesel's derive macros.
//! They serve as the boundary between the database and domain layers.

mod agent_session;
mod context_snapshot;
mod conversation;
mod domain_event;
mod handoff;
mod message;

pub use agent_session::{AgentSessionRow, NewAgentSession};
pub use context_snapshot::{ContextSnapshotRow, NewContextSnapshot};
pub use conversation::{ConversationRow, NewConversation};
pub use domain_event::{DomainEventRow, NewDomainEvent};
pub use handoff::{HandoffRow, NewHandoff};
pub use message::{MessageRow, NewMessage};
