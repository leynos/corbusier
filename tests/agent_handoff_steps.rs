//! BDD steps for agent handoff context preservation.
//!
//! Tests the complete handoff workflow using rstest-bdd.

use std::sync::Arc;

use corbusier::message::{
    adapters::memory::{
        InMemoryAgentSessionRepository, InMemoryContextSnapshotAdapter, InMemoryHandoffAdapter,
    },
    domain::{
        AgentSession, AgentSessionState, ConversationId, HandoffMetadata, HandoffStatus, MessageId,
        SequenceNumber, SnapshotType, ToolCallReference, TurnId,
    },
    ports::{
        agent_session::AgentSessionRepository, context_snapshot::ContextSnapshotPort,
        handoff::AgentHandoffPort,
    },
    services::HandoffService,
};
use eyre::{WrapErr, eyre};
use mockable::DefaultClock;
use rstest::fixture;
use rstest_bdd_macros::{given, scenario, then, when};

type TestHandoffService = HandoffService<
    InMemoryAgentSessionRepository,
    InMemoryHandoffAdapter<DefaultClock>,
    InMemoryContextSnapshotAdapter<DefaultClock>,
>;

/// World state for handoff BDD tests.
struct HandoffWorld {
    session_repo: Arc<InMemoryAgentSessionRepository>,
    handoff_adapter: Arc<InMemoryHandoffAdapter<DefaultClock>>,
    snapshot_adapter: Arc<InMemoryContextSnapshotAdapter<DefaultClock>>,
    service: TestHandoffService,
    conversation_id: ConversationId,
    source_session: Option<AgentSession>,
    target_session: Option<AgentSession>,
    current_handoff: Option<HandoffMetadata>,
    prior_turn_id: TurnId,
    tool_call_refs: Vec<ToolCallReference>,
    clock: DefaultClock,
}

impl Default for HandoffWorld {
    fn default() -> Self {
        let session_repo = Arc::new(InMemoryAgentSessionRepository::new());
        let handoff_adapter = Arc::new(InMemoryHandoffAdapter::new(DefaultClock));
        let snapshot_adapter = Arc::new(InMemoryContextSnapshotAdapter::new(DefaultClock));

        let service = HandoffService::new(
            Arc::clone(&session_repo),
            Arc::clone(&handoff_adapter),
            Arc::clone(&snapshot_adapter),
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
fn world() -> HandoffWorld {
    HandoffWorld::default()
}

fn run_async<T>(future: impl std::future::Future<Output = T>) -> T {
    tokio::task::block_in_place(|| tokio::runtime::Handle::current().block_on(future))
}

// ============================================================================
// Background Steps
// ============================================================================

#[given("an active agent session for a conversation")]
fn active_agent_session(world: &mut HandoffWorld) -> Result<(), eyre::Report> {
    let session = AgentSession::new(
        world.conversation_id,
        "source-agent",
        SequenceNumber::new(1),
        &world.clock,
    );

    run_async(world.session_repo.store(&session)).wrap_err("store session")?;

    world.source_session = Some(session);
    Ok(())
}

// ============================================================================
// Given Steps
// ============================================================================

#[given("an initiated handoff to a target agent")]
fn initiated_handoff(world: &mut HandoffWorld) -> Result<(), eyre::Report> {
    // Ensure we have an active session
    if world.source_session.is_none() {
        active_agent_session(world)?;
    }

    let source = world
        .source_session
        .as_ref()
        .ok_or_else(|| eyre!("no source session"))?;

    let handoff = run_async(world.service.initiate(
        world.conversation_id,
        source.session_id,
        "target-agent",
        world.prior_turn_id,
        SequenceNumber::new(5),
        Some("escalation needed"),
    ))
    .wrap_err("initiate handoff")?;

    world.current_handoff = Some(handoff);
    Ok(())
}

#[given("a conversation with tool calls")]
fn conversation_with_tool_calls(world: &mut HandoffWorld) -> Result<(), eyre::Report> {
    // Create source session
    active_agent_session(world)?;

    // Set up tool call references
    world.tool_call_refs = vec![
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
    ];

    Ok(())
}

#[given("a completed handoff from agent A to agent B")]
fn completed_handoff_a_to_b(world: &mut HandoffWorld) -> Result<(), eyre::Report> {
    // Use the existing source session as agent A (created by Background)
    // We need to update its agent_backend to "agent-A"
    let agent_a = if let Some(ref _existing) = world.source_session {
        // Use existing session, but we need to create a new one with correct agent name
        let session = AgentSession::new(
            world.conversation_id,
            "agent-A",
            SequenceNumber::new(1),
            &world.clock,
        );
        run_async(world.session_repo.store(&session)).wrap_err("store agent A")?;
        session
    } else {
        let agent_a = AgentSession::new(
            world.conversation_id,
            "agent-A",
            SequenceNumber::new(1),
            &world.clock,
        );
        run_async(world.session_repo.store(&agent_a)).wrap_err("store agent A")?;
        agent_a
    };

    // Initiate handoff
    let handoff = run_async(world.service.initiate(
        world.conversation_id,
        agent_a.session_id,
        "agent-B",
        TurnId::new(),
        SequenceNumber::new(5),
        None,
    ))
    .wrap_err("initiate A->B")?;

    // Create agent B session
    let agent_b = run_async(world.service.create_target_session(
        world.conversation_id,
        "agent-B",
        SequenceNumber::new(6),
        handoff.handoff_id,
    ))
    .wrap_err("create agent B")?;

    // Complete handoff
    run_async(world.service.complete(
        handoff.handoff_id,
        agent_b.session_id,
        SequenceNumber::new(6),
    ))
    .wrap_err("complete A->B")?;

    world.source_session = Some(agent_b);
    Ok(())
}

// ============================================================================
// When Steps
// ============================================================================

#[when("the current agent initiates a handoff to a specialist agent")]
fn initiate_specialist_handoff(world: &mut HandoffWorld) -> Result<(), eyre::Report> {
    let source = world
        .source_session
        .as_ref()
        .ok_or_else(|| eyre!("no source session"))?;

    let handoff = run_async(world.service.initiate(
        world.conversation_id,
        source.session_id,
        "specialist-agent",
        world.prior_turn_id,
        SequenceNumber::new(5),
        Some("task requires specialist"),
    ))
    .wrap_err("initiate handoff")?;

    world.current_handoff = Some(handoff);
    Ok(())
}

#[when("the target agent creates a new session")]
fn target_creates_session(world: &mut HandoffWorld) -> Result<(), eyre::Report> {
    let handoff = world
        .current_handoff
        .as_ref()
        .ok_or_else(|| eyre!("no current handoff"))?;

    let target = run_async(world.service.create_target_session(
        world.conversation_id,
        "target-agent",
        SequenceNumber::new(10),
        handoff.handoff_id,
    ))
    .wrap_err("create target session")?;

    world.target_session = Some(target);
    Ok(())
}

#[when("the handoff is completed")]
fn complete_handoff(world: &mut HandoffWorld) -> Result<(), eyre::Report> {
    let handoff = world
        .current_handoff
        .as_ref()
        .ok_or_else(|| eyre!("no current handoff"))?;

    let target = world
        .target_session
        .as_ref()
        .ok_or_else(|| eyre!("no target session"))?;

    let completed = run_async(world.service.complete(
        handoff.handoff_id,
        target.session_id,
        SequenceNumber::new(10),
    ))
    .wrap_err("complete handoff")?;

    world.current_handoff = Some(completed);
    Ok(())
}

#[when("the handoff is cancelled")]
fn cancel_handoff(world: &mut HandoffWorld) -> Result<(), eyre::Report> {
    let handoff = world
        .current_handoff
        .as_ref()
        .ok_or_else(|| eyre!("no current handoff"))?;

    run_async(
        world
            .service
            .cancel(handoff.handoff_id, Some("target unavailable")),
    )
    .wrap_err("cancel handoff")?;

    Ok(())
}

#[when("a handoff is initiated with tool call references")]
fn initiate_with_tool_calls(world: &mut HandoffWorld) -> Result<(), eyre::Report> {
    let source = world
        .source_session
        .as_ref()
        .ok_or_else(|| eyre!("no source session"))?;

    let mut handoff = run_async(world.service.initiate(
        world.conversation_id,
        source.session_id,
        "specialist-agent",
        world.prior_turn_id,
        SequenceNumber::new(5),
        Some("tool results need review"),
    ))
    .wrap_err("initiate handoff")?;

    // Add tool call references
    for tcr in &world.tool_call_refs {
        handoff = handoff.with_triggering_tool_call(tcr.clone());
    }

    world.current_handoff = Some(handoff);
    Ok(())
}

#[when("agent B initiates a handoff to agent C")]
fn agent_b_to_c(world: &mut HandoffWorld) -> Result<(), eyre::Report> {
    let agent_b = world
        .source_session
        .as_ref()
        .ok_or_else(|| eyre!("no agent B session"))?;

    let handoff = run_async(world.service.initiate(
        world.conversation_id,
        agent_b.session_id,
        "agent-C",
        TurnId::new(),
        SequenceNumber::new(10),
        Some("need domain expert"),
    ))
    .wrap_err("initiate B->C")?;

    // Create agent C
    let agent_c = run_async(world.service.create_target_session(
        world.conversation_id,
        "agent-C",
        SequenceNumber::new(11),
        handoff.handoff_id,
    ))
    .wrap_err("create agent C")?;

    // Complete handoff
    run_async(world.service.complete(
        handoff.handoff_id,
        agent_c.session_id,
        SequenceNumber::new(11),
    ))
    .wrap_err("complete B->C")?;

    world.target_session = Some(agent_c);
    world.current_handoff = Some(handoff);
    Ok(())
}

// ============================================================================
// Then Steps
// ============================================================================

#[then("a handoff record is created with initiated status")]
fn handoff_initiated_status(world: &HandoffWorld) -> Result<(), eyre::Report> {
    let handoff = world
        .current_handoff
        .as_ref()
        .ok_or_else(|| eyre!("no handoff"))?;

    if handoff.status != HandoffStatus::Initiated {
        return Err(eyre!("expected Initiated status, got {:?}", handoff.status));
    }
    Ok(())
}

#[then("a context snapshot is captured for the source session")]
fn snapshot_captured(world: &HandoffWorld) -> Result<(), eyre::Report> {
    let source = world
        .source_session
        .as_ref()
        .ok_or_else(|| eyre!("no source session"))?;

    let snapshots = run_async(
        world
            .snapshot_adapter
            .find_snapshots_for_session(source.session_id),
    )
    .wrap_err("find snapshots")?;

    if snapshots.is_empty() {
        return Err(eyre!("no snapshots captured"));
    }

    let has_handoff_snapshot = snapshots
        .iter()
        .any(|s| s.snapshot_type == SnapshotType::HandoffInitiated);

    if !has_handoff_snapshot {
        return Err(eyre!("no HandoffInitiated snapshot found"));
    }

    Ok(())
}

#[then("the source session is marked as handed off")]
fn source_handed_off(world: &HandoffWorld) -> Result<(), eyre::Report> {
    let source = world
        .source_session
        .as_ref()
        .ok_or_else(|| eyre!("no source session"))?;

    let updated =
        run_async(world.session_repo.find_by_id(source.session_id)).wrap_err("find source")?;

    let session = updated.ok_or_else(|| eyre!("source session not found"))?;

    if session.state != AgentSessionState::HandedOff {
        return Err(eyre!("expected HandedOff state, got {:?}", session.state));
    }

    Ok(())
}

#[then("the handoff record links source and target sessions")]
fn handoff_links_sessions(world: &HandoffWorld) -> Result<(), eyre::Report> {
    let handoff = world
        .current_handoff
        .as_ref()
        .ok_or_else(|| eyre!("no handoff"))?;

    let target = world
        .target_session
        .as_ref()
        .ok_or_else(|| eyre!("no target session"))?;

    if handoff.target_session_id != Some(target.session_id) {
        return Err(eyre!("handoff does not link to target session"));
    }

    Ok(())
}

#[then("the handoff status is completed")]
fn handoff_completed(world: &HandoffWorld) -> Result<(), eyre::Report> {
    let handoff = world
        .current_handoff
        .as_ref()
        .ok_or_else(|| eyre!("no handoff"))?;

    if handoff.status != HandoffStatus::Completed {
        return Err(eyre!("expected Completed status, got {:?}", handoff.status));
    }

    if handoff.completed_at.is_none() {
        return Err(eyre!("completed_at should be set"));
    }

    Ok(())
}

#[then("the source session is reverted to active state")]
fn source_reverted(world: &HandoffWorld) -> Result<(), eyre::Report> {
    let source = world
        .source_session
        .as_ref()
        .ok_or_else(|| eyre!("no source session"))?;

    let updated =
        run_async(world.session_repo.find_by_id(source.session_id)).wrap_err("find source")?;

    let session = updated.ok_or_else(|| eyre!("source session not found"))?;

    if session.state != AgentSessionState::Active {
        return Err(eyre!("expected Active state, got {:?}", session.state));
    }

    if session.terminated_by_handoff.is_some() {
        return Err(eyre!("terminated_by_handoff should be None"));
    }

    Ok(())
}

#[then("no target session is created")]
fn no_target_session(world: &HandoffWorld) -> Result<(), eyre::Report> {
    if world.target_session.is_some() {
        return Err(eyre!("target session should not exist"));
    }
    Ok(())
}

#[then("the handoff metadata includes the prior turn id")]
fn handoff_includes_turn(world: &HandoffWorld) -> Result<(), eyre::Report> {
    let handoff = world
        .current_handoff
        .as_ref()
        .ok_or_else(|| eyre!("no handoff"))?;

    if handoff.prior_turn_id != world.prior_turn_id {
        return Err(eyre!("prior_turn_id does not match"));
    }

    Ok(())
}

#[then("the handoff metadata includes the triggering tool calls")]
fn handoff_includes_tool_calls(world: &HandoffWorld) -> Result<(), eyre::Report> {
    let handoff = world
        .current_handoff
        .as_ref()
        .ok_or_else(|| eyre!("no handoff"))?;

    if handoff.triggering_tool_calls.len() != world.tool_call_refs.len() {
        return Err(eyre!(
            "expected {} tool calls, got {}",
            world.tool_call_refs.len(),
            handoff.triggering_tool_calls.len()
        ));
    }

    Ok(())
}

#[then("the conversation history shows all agent sessions")]
fn history_shows_all_sessions(world: &HandoffWorld) -> Result<(), eyre::Report> {
    let sessions = run_async(
        world
            .session_repo
            .find_by_conversation(world.conversation_id),
    )
    .wrap_err("list sessions")?;

    // Should have at least A, B, and C (may also include background session)
    // The background step creates a "source-agent" session
    if sessions.len() < 3 {
        return Err(eyre!(
            "expected at least 3 sessions, got {}",
            sessions.len()
        ));
    }

    let agents: Vec<&str> = sessions.iter().map(|s| s.agent_backend.as_str()).collect();

    for expected in &["agent-A", "agent-B", "agent-C"] {
        if !agents.contains(expected) {
            return Err(eyre!("missing agent: {expected}"));
        }
    }

    Ok(())
}

#[then("each handoff is linked in sequence")]
fn handoffs_linked_in_sequence(world: &HandoffWorld) -> Result<(), eyre::Report> {
    let handoffs = run_async(
        world
            .handoff_adapter
            .list_handoffs_for_conversation(world.conversation_id),
    )
    .wrap_err("list handoffs")?;

    if handoffs.len() != 2 {
        return Err(eyre!("expected 2 handoffs, got {}", handoffs.len()));
    }

    // All should be completed
    for handoff in &handoffs {
        if handoff.status != HandoffStatus::Completed {
            return Err(eyre!("handoff {:?} is not completed", handoff.handoff_id));
        }
    }

    Ok(())
}

// ============================================================================
// Scenario Definitions
// ============================================================================

#[scenario(
    path = "tests/features/agent_handoff.feature",
    name = "Successful handoff to a different agent"
)]
#[tokio::test(flavor = "multi_thread")]
async fn successful_handoff(world: HandoffWorld) {
    let _ = world;
}

#[scenario(
    path = "tests/features/agent_handoff.feature",
    name = "Complete handoff when target agent accepts"
)]
#[tokio::test(flavor = "multi_thread")]
async fn complete_handoff_scenario(world: HandoffWorld) {
    let _ = world;
}

#[scenario(
    path = "tests/features/agent_handoff.feature",
    name = "Cancel a pending handoff"
)]
#[tokio::test(flavor = "multi_thread")]
async fn cancel_handoff_scenario(world: HandoffWorld) {
    let _ = world;
}

#[scenario(
    path = "tests/features/agent_handoff.feature",
    name = "Handoff references prior turn and tool calls"
)]
#[tokio::test(flavor = "multi_thread")]
async fn handoff_with_references(world: HandoffWorld) {
    let _ = world;
}

#[scenario(
    path = "tests/features/agent_handoff.feature",
    name = "Multiple handoffs in a conversation chain"
)]
#[tokio::test(flavor = "multi_thread")]
async fn multiple_handoffs(world: HandoffWorld) {
    let _ = world;
}
