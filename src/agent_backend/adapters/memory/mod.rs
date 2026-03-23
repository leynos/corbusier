//! In-memory adapters for agent backend orchestration.

mod backend_registry;
mod runtime;
mod tool_router;
mod turn_session;

pub use backend_registry::InMemoryBackendRegistry;
pub use runtime::{InMemoryAgentRuntime, RuntimeExecutionRecord};
pub use tool_router::InMemoryToolRouter;
pub use turn_session::InMemoryTurnSessionRepository;
