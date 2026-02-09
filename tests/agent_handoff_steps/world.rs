//! World state for handoff BDD scenarios.

use std::sync::Arc;

use corbusier::message::{
    adapters::memory::{
        InMemoryAgentSessionRepository, InMemoryContextSnapshotAdapter, InMemoryHandoffAdapter,
    },
    domain::{
        AgentSession, ConversationId, HandoffMetadata, MessageId, SequenceNumber,
        ToolCallReference, TurnId,
    },
    ports::{
        agent_session::AgentSessionRepository,
    },
    services::HandoffService,
};
use eyre::WrapErr;
use mockable::DefaultClock;
use rstest::fixture;

pub type TestHandoffService = HandoffService<
    InMemoryAgentSessionRepository,
    InMemoryHandoffAdapter<DefaultClock>,
    InMemoryContextSnapshotAdapter,
    DefaultClock,
>;

/// World state for handoff BDD tests.
pub struct HandoffWorld {
    pub session_repo: Arc<InMemoryAgentSessionRepository>,
    pub handoff_adapter: Arc<InMemoryHandoffAdapter<DefaultClock>>,
    pub snapshot_adapter: Arc<InMemoryContextSnapshotAdapter>,
    pub service: TestHandoffService,
    pub conversation_id: ConversationId,
    pub source_session: Option<AgentSession>,
    pub target_session: Option<AgentSession>,
    pub current_handoff: Option<HandoffMetadata>,
    pub prior_turn_id: TurnId,
    pub tool_call_refs: Vec<ToolCallReference>,
    pub clock: DefaultClock,
}

impl Default for HandoffWorld {
    fn default() -> Self {
        let service_clock = Arc::new(DefaultClock);
        let session_repo = Arc::new(InMemoryAgentSessionRepository::new());
        let handoff_adapter = Arc::new(InMemoryHandoffAdapter::new(DefaultClock));
        let snapshot_adapter = Arc::new(InMemoryContextSnapshotAdapter::new());

        let service = HandoffService::new(
            Arc::clone(&session_repo),
            Arc::clone(&handoff_adapter),
            Arc::clone(&snapshot_adapter),
            service_clock,
        );

        Self {
            session_repo,
            handoff_adapter,
            snapshot_adapter,
            service,
            conversation_id: ConversationId::new(),
            source_session: None,
            target_session: None,
            current_handoff: None,
            prior_turn_id: TurnId::new(),
            tool_call_refs: Vec::new(),
            clock: DefaultClock,
        }
    }
}

#[fixture]
pub fn world() -> HandoffWorld {
    HandoffWorld::default()
}

pub fn run_async<T>(future: impl std::future::Future<Output = T>) -> T {
    tokio::task::block_in_place(|| tokio::runtime::Handle::current().block_on(future))
}

pub fn create_and_store_session(
    world: &HandoffWorld,
    agent_backend: &str,
    start_sequence: SequenceNumber,
) -> Result<AgentSession, eyre::Report> {
    let session =
        AgentSession::new(world.conversation_id, agent_backend, start_sequence, &world.clock);
    run_async(world.session_repo.store(&session)).wrap_err("store session")?;
    Ok(session)
}

pub fn create_tool_call_refs() -> Vec<ToolCallReference> {
    vec![
        ToolCallReference::new(
            "call-001",
            "read_file",
            MessageId::new(),
            SequenceNumber::new(3),
        ),
        ToolCallReference::new(
            "call-002",
            "search",
            MessageId::new(),
            SequenceNumber::new(4),
        ),
    ]
}
