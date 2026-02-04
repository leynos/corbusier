# Execution Plan: Preserve Context Across Agent Handoffs (1.1.3)

## Objective

Implement roadmap item 1.1.3 from `docs/roadmap.md`:

- Persist handoff metadata between agent turns
- Maintain context window snapshots per agent session
- **Success criteria**: Every handoff references the prior turn and tool calls
  used to reach the handoff

## References

- Design: `docs/corbusier-design.md` §2.2.1, §4.1.1.1, §4.2.1.1
- Testing: `docs/rstest-bdd-users-guide.md`,
  `docs/pg-embed-setup-unpriv-users-guide.md`
- Architecture: Hexagonal (domain/ports/adapters separation)

______________________________________________________________________

## Phase 1: Domain Types (src/message/domain/)

### 1.1 Add new identifier types to `ids.rs`

Add the following, using the existing `TurnId` pattern (~30 lines each):

```rust
pub struct HandoffId(Uuid);       // Unique handoff event identifier
pub struct AgentSessionId(Uuid);  // Agent session identifier
```

Each with: `new()`, `from_uuid()`, `into_inner()`, `Default`, `AsRef<Uuid>`,
`Display`.

### 1.2 Create `handoff.rs` (~150 lines)

```rust
pub struct HandoffMetadata {
    pub handoff_id: HandoffId,
    pub source_session_id: AgentSessionId,
    pub target_session_id: Option<AgentSessionId>,
    pub prior_turn_id: TurnId,
    pub triggering_tool_calls: Vec<ToolCallReference>,
    pub source_agent: String,
    pub target_agent: String,
    pub reason: Option<String>,
    pub initiated_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
    pub status: HandoffStatus,
}

pub struct ToolCallReference {
    pub call_id: String,
    pub tool_name: String,
    pub message_id: MessageId,
    pub sequence_number: SequenceNumber,
}

pub enum HandoffStatus { Initiated, Accepted, Completed, Failed, Cancelled }
```

Builder pattern: `HandoffMetadata::new()`, `with_triggering_tool_call()`,
`with_reason()`, `complete()`.

### 1.3 Create `context_snapshot.rs` (~120 lines)

```rust
pub struct ContextWindowSnapshot {
    pub snapshot_id: Uuid,
    pub conversation_id: ConversationId,
    pub session_id: AgentSessionId,
    pub sequence_range: SequenceRange,
    pub message_summary: MessageSummary,
    pub visible_tool_calls: Vec<ToolCallReference>,
    pub token_estimate: Option<u64>,
    pub captured_at: DateTime<Utc>,
    pub snapshot_type: SnapshotType,
}

pub struct SequenceRange { pub start: SequenceNumber, pub end: SequenceNumber }
pub struct MessageSummary { pub user_count: u32, pub assistant_count: u32, ... }
pub enum SnapshotType { SessionStart, HandoffInitiated, Truncation, Checkpoint }
```

### 1.4 Create `agent_session.rs` (~130 lines)

```rust
pub struct AgentSession {
    pub session_id: AgentSessionId,
    pub conversation_id: ConversationId,
    pub agent_backend: String,
    pub start_sequence: SequenceNumber,
    pub end_sequence: Option<SequenceNumber>,
    pub turn_ids: Vec<TurnId>,
    pub initiated_by_handoff: Option<HandoffId>,
    pub terminated_by_handoff: Option<HandoffId>,
    pub context_snapshots: Vec<ContextWindowSnapshot>,
    pub started_at: DateTime<Utc>,
    pub ended_at: Option<DateTime<Utc>>,
    pub state: AgentSessionState,
}

pub enum AgentSessionState { Active, Paused, HandedOff, Completed, Failed }
```

### 1.5 Extend `metadata.rs`

Add two fields to `MessageMetadata`:

```rust
pub handoff_metadata: Option<HandoffMetadata>,
pub agent_session_id: Option<AgentSessionId>,
```

Add builders: `with_handoff_metadata()`, `with_agent_session_id()`.

### 1.6 Update `domain/mod.rs`

Export new modules and types.

______________________________________________________________________

## Phase 2: Port Definitions (src/message/ports/)

### 2.1 Create `handoff.rs` (~80 lines)

```rust
#[async_trait]
pub trait AgentHandoffPort: Send + Sync {
    async fn initiate_handoff(...) -> HandoffResult<HandoffMetadata>;
    async fn complete_handoff(...) -> HandoffResult<HandoffMetadata>;
    async fn cancel_handoff(...) -> HandoffResult<()>;
    async fn find_handoff(...) -> HandoffResult<Option<HandoffMetadata>>;
    async fn list_handoffs_for_conversation(...) -> HandoffResult<Vec<HandoffMetadata>>;
}

pub enum HandoffError { NotFound, InvalidStateTransition, SessionNotFound, ... }
```

### 2.2 Create `context_snapshot.rs` (~50 lines)

```rust
#[async_trait]
pub trait ContextSnapshotPort: Send + Sync {
    async fn capture_snapshot(...) -> SnapshotResult<ContextWindowSnapshot>;
    async fn find_snapshots_for_session(...) -> SnapshotResult<Vec<...>>;
    async fn find_latest_snapshot(...) -> SnapshotResult<Option<...>>;
}
```

### 2.3 Create `agent_session.rs` (~60 lines)

```rust
#[async_trait]
pub trait AgentSessionRepository: Send + Sync {
    async fn store(&self, session: &AgentSession) -> SessionResult<()>;
    async fn update(&self, session: &AgentSession) -> SessionResult<()>;
    async fn find_by_id(...) -> SessionResult<Option<AgentSession>>;
    async fn find_active_for_conversation(...) -> SessionResult<Option<AgentSession>>;
    async fn find_by_conversation(...) -> SessionResult<Vec<AgentSession>>;
}
```

### 2.4 Update `ports/mod.rs`

Export new port modules.

______________________________________________________________________

## Phase 3: In-Memory Adapters (src/message/adapters/memory/)

### 3.1 Create `agent_session.rs` (~100 lines)

`InMemoryAgentSessionRepository` using `Arc<RwLock<HashMap>>` pattern from
existing `memory.rs`.

### 3.2 Create `handoff.rs` (~120 lines)

`InMemoryHandoffAdapter` implementing `AgentHandoffPort`.

### 3.3 Create `context_snapshot.rs` (~80 lines)

`InMemoryContextSnapshotAdapter` implementing `ContextSnapshotPort`.

### 3.4 Restructure memory adapters

Move existing `memory.rs` content to `memory/message.rs`, create
`memory/mod.rs` exporting all in-memory adapters.

______________________________________________________________________

## Phase 4: Database Schema (migrations/)

### 4.1 Create migration `2026-02-03-000000_add_agent_sessions_and_handoffs`

**up.sql:**

```sql
CREATE TABLE agent_sessions (
    id UUID PRIMARY KEY,
    conversation_id UUID NOT NULL REFERENCES conversations(id),
    agent_backend VARCHAR(100) NOT NULL,
    start_sequence BIGINT NOT NULL,
    end_sequence BIGINT,
    turn_ids JSONB NOT NULL DEFAULT '[]',
    initiated_by_handoff UUID,
    terminated_by_handoff UUID,
    context_snapshots JSONB NOT NULL DEFAULT '[]',
    started_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    ended_at TIMESTAMPTZ,
    state VARCHAR(20) NOT NULL DEFAULT 'active'
);

CREATE TABLE handoffs (
    id UUID PRIMARY KEY,
    conversation_id UUID NOT NULL REFERENCES conversations(id),
    source_session_id UUID NOT NULL REFERENCES agent_sessions(id),
    target_session_id UUID REFERENCES agent_sessions(id),
    prior_turn_id UUID NOT NULL,
    triggering_tool_calls JSONB NOT NULL DEFAULT '[]',
    source_agent VARCHAR(100) NOT NULL,
    target_agent VARCHAR(100) NOT NULL,
    reason TEXT,
    initiated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    completed_at TIMESTAMPTZ,
    status VARCHAR(20) NOT NULL DEFAULT 'initiated'
);

CREATE TABLE context_snapshots (
    id UUID PRIMARY KEY,
    conversation_id UUID NOT NULL REFERENCES conversations(id),
    session_id UUID NOT NULL REFERENCES agent_sessions(id),
    sequence_start BIGINT NOT NULL,
    sequence_end BIGINT NOT NULL,
    message_summary JSONB NOT NULL,
    visible_tool_calls JSONB NOT NULL DEFAULT '[]',
    token_estimate BIGINT,
    captured_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    snapshot_type VARCHAR(30) NOT NULL
);

-- Indexes
CREATE INDEX idx_agent_sessions_conversation ON agent_sessions(conversation_id);
CREATE INDEX idx_agent_sessions_active ON agent_sessions(conversation_id, state)
    WHERE state = 'active';
CREATE INDEX idx_handoffs_conversation ON handoffs(conversation_id);
CREATE INDEX idx_context_snapshots_session ON context_snapshots(session_id);
```

### 4.2 Update `adapters/schema.rs`

Run `diesel print-schema` to regenerate table definitions.

### 4.3 Create `adapters/models/session_models.rs` (~150 lines)

Diesel models: `AgentSessionRow`, `NewAgentSession`, `HandoffRow`,
`NewHandoff`, `ContextSnapshotRow`, `NewContextSnapshot`.

______________________________________________________________________

## Phase 5: PostgreSQL Adapters (src/message/adapters/postgres/)

### 5.1 Create `agent_session.rs` (~150 lines)

`PostgresAgentSessionRepository` following existing `PostgresMessageRepository`
pattern with `spawn_blocking` for Diesel operations.

### 5.2 Create `handoff.rs` (~180 lines)

`PostgresHandoffAdapter` implementing `AgentHandoffPort` with transaction
support.

### 5.3 Create `context_snapshot.rs` (~120 lines)

`PostgresContextSnapshotAdapter` implementing `ContextSnapshotPort`.

### 5.4 Update `postgres/mod.rs`

Export new adapters.

______________________________________________________________________

## Phase 6: Service Layer (src/message/services/)

### 6.1 Create `services/mod.rs`

### 6.2 Create `services/handoff_service.rs` (~200 lines)

```rust
pub struct HandoffService<H, S, C, M> { ... }

impl HandoffService {
    pub async fn initiate_handoff(...) -> Result<HandoffMetadata, ...>;
    pub async fn complete_handoff(...) -> Result<(HandoffMetadata, AgentSession), ...>;
    pub async fn create_session(...) -> Result<AgentSession, ...>;
}
```

Orchestrates: find active session -> capture snapshot -> collect tool refs ->
create handoff -> update session state.

______________________________________________________________________

## Phase 7: Validation (src/message/validation/)

### 7.1 Create `handoff_rules.rs` (~80 lines)

- `validate_prior_turn_reference()` - verify turn exists in conversation
- `validate_tool_call_references()` - verify tool calls exist
- `validate_handoff_transition()` - valid state transitions
- `validate_session_transition()` - valid state transitions

### 7.2 Extend `error.rs`

Add `HandoffValidationError` enum.

______________________________________________________________________

## Phase 8: Testing

### 8.1 Unit tests (`src/message/tests/`)

| File                          | Coverage                                       |
| ----------------------------- | ---------------------------------------------- |
| `handoff_tests.rs`            | `HandoffMetadata` construction, builder, serde |
| `agent_session_tests.rs`      | `AgentSession` lifecycle, state transitions    |
| `context_snapshot_tests.rs`   | Snapshot construction, `MessageSummary`        |
| `handoff_validation_tests.rs` | Validation rules happy/unhappy paths           |

### 8.2 Integration tests (`tests/`)

| File                         | Coverage                                  |
| ---------------------------- | ----------------------------------------- |
| `in_memory/handoff_tests.rs` | Full handoff flow with in-memory adapters |
| `postgres/handoff_tests.rs`  | Persistence, audit trail, transactions    |

### 8.3 Behaviour-Driven Development (BDD) tests

**`tests/features/agent_handoff.feature`:**

```gherkin
Feature: Agent handoff context preservation

  Scenario: Successful agent handoff preserves context
    Given a conversation with agent session "claude-code"
    And the session has processed 3 turns with tool calls
    When a handoff is initiated to "opus-agent"
    Then the handoff references the prior turn
    And the handoff includes tool call references
    And a context snapshot is captured
    When the target agent accepts the handoff
    Then a new session is created for "opus-agent"
    And the handoff status is "completed"

  Scenario: Context snapshot captures message summary
    Given a conversation with 5 user messages and 5 assistant messages
    When a context snapshot is captured
    Then the snapshot message summary shows 5 user and 5 assistant messages
```

**`tests/agent_handoff_steps.rs`:** Step definitions using `rstest_bdd_macros`.

______________________________________________________________________

## Phase 9: Documentation

### 9.1 Update `docs/users-guide.md`

Add section covering:

- Agent session tracking
- Handoff metadata structure
- Context window snapshots
- Querying handoff history

### 9.2 Update `docs/corbusier-design.md`

Document design decisions:

- Schema choices (JSONB for nested structures)
- State transition rules
- Snapshot capture timing

### 9.3 Update `docs/roadmap.md`

Mark item 1.1.3 and sub-items as `[x]` complete.

______________________________________________________________________

## Implementation Order

| Step | Task                         | Files                                          | Est. Lines |
| ---- | ---------------------------- | ---------------------------------------------- | ---------- |
| 1    | Domain identifiers           | `domain/ids.rs`                                | +60        |
| 2    | Handoff types                | `domain/handoff.rs` (new)                      | +150       |
| 3    | Context snapshot types       | `domain/context_snapshot.rs` (new)             | +120       |
| 4    | Agent session types          | `domain/agent_session.rs` (new)                | +130       |
| 5    | Extend MessageMetadata       | `domain/metadata.rs`                           | +30        |
| 6    | Domain module exports        | `domain/mod.rs`                                | +10        |
| 7    | Unit tests for domain        | `tests/*.rs`                                   | +300       |
| 8    | Port definitions             | `ports/*.rs` (3 new files)                     | +200       |
| 9    | In-memory adapters           | `adapters/memory/*.rs`                         | +300       |
| 10   | In-memory integration tests  | `tests/in_memory/*.rs`                         | +200       |
| 11   | Database migration           | `migrations/*/up.sql`                          | +50        |
| 12   | Diesel schema + models       | `adapters/schema.rs`, `models/*.rs`            | +200       |
| 13   | PostgreSQL adapters          | `adapters/postgres/*.rs`                       | +450       |
| 14   | PostgreSQL integration tests | `tests/postgres/*.rs`                          | +250       |
| 15   | Validation rules             | `validation/handoff_rules.rs`                  | +100       |
| 16   | Service layer                | `services/handoff_service.rs`                  | +200       |
| 17   | BDD feature + steps          | `tests/features/*.feature`, `tests/*_steps.rs` | +150       |
| 18   | Documentation updates        | `docs/*.md`                                    | +100       |
| 19   | Run quality gates            | `make check-fmt && make lint && make test`     | -          |
| 20   | Mark roadmap complete        | `docs/roadmap.md`                              | +3         |

**Total estimated**: ~3000 lines across ~25 files

______________________________________________________________________

## Critical File Paths

**Domain (modify/create):**

- `/home/user/project/src/message/domain/ids.rs`
- `/home/user/project/src/message/domain/handoff.rs` (new)
- `/home/user/project/src/message/domain/context_snapshot.rs` (new)
- `/home/user/project/src/message/domain/agent_session.rs` (new)
- `/home/user/project/src/message/domain/metadata.rs`
- `/home/user/project/src/message/domain/mod.rs`

**Ports (create):**

- `/home/user/project/src/message/ports/handoff.rs` (new)
- `/home/user/project/src/message/ports/context_snapshot.rs` (new)
- `/home/user/project/src/message/ports/agent_session.rs` (new)

**Adapters (create):**

- `/home/user/project/src/message/adapters/memory/` (restructure + new files)
- `/home/user/project/src/message/adapters/postgres/agent_session.rs` (new)
- `/home/user/project/src/message/adapters/postgres/handoff.rs` (new)
- `/home/user/project/src/message/adapters/postgres/context_snapshot.rs` (new)
- `/home/user/project/src/message/adapters/models/session_models.rs` (new)

**Database:**

- `/home/user/project/migrations/2026-02-03-000000_add_agent_sessions_and_handoffs/`

**Services (create):**

- `/home/user/project/src/message/services/mod.rs` (new)
- `/home/user/project/src/message/services/handoff_service.rs` (new)

**Tests (create):**

- `/home/user/project/src/message/tests/handoff_tests.rs` (new)
- `/home/user/project/src/message/tests/agent_session_tests.rs` (new)
- `/home/user/project/src/message/tests/context_snapshot_tests.rs` (new)
- `/home/user/project/tests/in_memory/handoff_tests.rs` (new)
- `/home/user/project/tests/postgres/handoff_tests.rs` (new)
- `/home/user/project/tests/features/agent_handoff.feature` (new)
- `/home/user/project/tests/agent_handoff_steps.rs` (new)

**Documentation (modify):**

- `/home/user/project/docs/users-guide.md`
- `/home/user/project/docs/corbusier-design.md`
- `/home/user/project/docs/roadmap.md`

______________________________________________________________________

## Quality Gates (before commit)

```bash
make check-fmt && make lint && make test
```

All must pass before marking roadmap item complete.

______________________________________________________________________

## Risks and Mitigations

| Risk                           | Mitigation                                              |
| ------------------------------ | ------------------------------------------------------- |
| Schema complexity              | Use JSONB for nested structures to minimize table count |
| Performance (snapshot capture) | Make snapshot async, use token estimates                |
| State machine complexity       | Explicit enums with validation functions                |
| File size limits (400 lines)   | Split large modules into submodules                     |

______________________________________________________________________

## Design Decisions to Record

1. **Handoff metadata stored in MessageMetadata**: Enables full audit trail via
   existing message persistence.
2. **JSONB for tool call references and snapshots**: Flexible schema, avoids
   join-heavy queries.
3. **AgentSession as first-class entity**: Enables session-level queries and
   lifecycle tracking.
4. **Snapshot types enum**: Distinguishes capture contexts for debugging and
   analysis.
