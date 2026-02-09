//! In-memory adapter implementations for testing.
//!
//! These adapters provide simple, thread-safe implementations suitable for
//! unit testing without database dependencies.

mod agent_session;
mod context_snapshot;
mod handoff;
mod message;

pub use agent_session::InMemoryAgentSessionRepository;
pub use context_snapshot::InMemoryContextSnapshotAdapter;
pub use handoff::InMemoryHandoffAdapter;
pub use message::InMemoryMessageRepository;
