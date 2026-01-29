# Persist Message History with Audit Trails (Roadmap 1.1.2)

This Execution Plan (ExecPlan) is a living document. The sections
`Constraints`, `Tolerances`, `Risks`, `Progress`, `Surprizes & Discoveries`,
`Decision Log`, and `Outcomes & Retrospective` must be kept up to date as work
proceeds.

Status: COMPLETE

No PLANS.md exists in this repository at the time of writing.

This document must be maintained in accordance with the execplans skill located
at `/root/.codex/skills/execplans/SKILL.md`.

## Purpose / Big Picture

After this change, Corbusier will persist conversation message history with
immutable ordering and complete audit metadata for tool calls and agent
responses. A user (or service) will be able to query conversation history by
conversation id and receive messages in sequence order, with metadata that
captures tool call provenance and agent response context for audit trails.

Observable success: unit tests validate metadata shape, message ordering, and
error handling; behavioural tests demonstrate end-to-end persistence and
retrieval with audit metadata; Postgres-backed integration tests pass using the
embedded cluster helper.

## Constraints

Hard invariants that must not be violated:

- **Hexagonal architecture**: domain types must not import adapters or Diesel
  types. Adapters implement ports; adapters must not call each other directly.
- **Immutable history**: messages are append-only and must not be updated or
  deleted after persistence per corbusier-design.md §2.2.1 and §6.2.3.
- **File size limit**: no source file may exceed 400 lines (AGENTS.md).
- **No panic-prone code**: Clippy denies `unwrap_used`, `expect_used`,
  `panic_in_result_fn`, and related lints.
- **Audit metadata coverage**: tool call and agent response audit metadata must
  be persisted and retrievable with the message history.
- **Testing discipline**: unit tests use `rstest`; behavioural tests use
  `rstest-bdd` v0.4.0 where applicable; Postgres tests use
  `pg-embed-setup-unpriv`.
- **Documentation**: new public APIs have rustdoc with examples; module files
  begin with `//!` comments; docs are en-GB-oxendict and wrapped at 80 columns.

## Tolerances (Exception Triggers)

Stop and escalate if any threshold is exceeded:

- **Scope**: more than 16 files or 1500 net LOC changed.
- **Dependencies**: any new runtime dependency, or more than two new
  dev-dependencies beyond `rstest-bdd`/`rstest-bdd-macros`.
- **Schema**: a new database table or migration is required (beyond metadata
  JSONB updates and existing audit triggers).
- **Interfaces**: a public API signature must change in a way that breaks
  existing callers.
- **Tests**: any test suite fails after three fix attempts.
- **Ambiguity**: multiple valid interpretations of audit metadata shape that
  would materially change persisted JSON or downstream consumption.

## Risks

- Risk: audit metadata fields are underspecified in the design doc, leading to
  incompatible shapes. Severity: medium; Likelihood: medium; Mitigation: derive
  a minimal schema from corbusier-design.md §2.2.1 and §6.2.1.2, record the
  decision in the design document, and keep metadata extensible.

- Risk: race conditions when allocating sequence numbers under concurrency.
  Severity: medium; Likelihood: low; Mitigation: use repository-level
  transaction or retry logic, and lean on the unique constraint for safety.

- Risk: `rstest-bdd` v0.4.0 availability or async support mismatch.
  Severity: medium; Likelihood: medium; Mitigation: confirm dependency
  availability early and use `#[tokio::test]` on scenario functions per
  docs/rstest-bdd-users-guide.md.

- Risk: Postgres embedded cluster boot time increases test runtime.
  Severity: low; Likelihood: medium; Mitigation: reuse template database via
  existing helpers in `tests/postgres/cluster` and limit BDD scenarios.

## Progress

- [x] (2026-01-28 00:00Z) Drafted ExecPlan.
- [x] (2026-01-28 00:00Z) Stage A: requirements alignment and audit metadata
  schema decision.
- [x] (2026-01-28 00:00Z) Stage B: domain model and ports for conversation
  history + audit metadata.
- [x] (2026-01-28 00:00Z) Stage C: adapters and persistence updates (Postgres +
  in-memory).
- [x] (2026-01-28 00:00Z) Stage D: unit, integration, and BDD tests.
- [x] (2026-01-28 00:00Z) Stage E: documentation, design notes, roadmap update,
  and quality gates.

## Surprizes & Discoveries

- Observation: Message persistence, audit triggers, and Postgres integration
  helpers already exist under `src/message/adapters` and `tests/postgres`.
  Evidence: `PostgresMessageRepository::store_with_audit` and migrations in
  `migrations/2026-01-16-000000_add_audit_trigger`. Impact: Implementation can
  extend existing ports and tests rather than adding new infrastructure from
  scratch.

## Decision Log

- Decision: Keep the feature-based layout under `src/message/` and add new
  modules there (rather than introducing a separate `application/` tree).
  Rationale: Aligns with AGENTS.md “group by feature, not layer” guidance while
  preserving hexagonal boundaries. Date/Author: 2026-01-28, Codex.

- Decision: Persist tool call and agent response audit metadata inside
  `MessageMetadata` (JSONB) with typed structs and explicit optional fields.
  Rationale: Matches corbusier-design.md §5.2.1 and §6.2.1.2, avoids new
  tables, and keeps message retrieval self-contained. Date/Author: 2026-01-28,
  Codex.

- Decision: Standardise audit status values in metadata. Tool calls use
  `queued`, `running`, `succeeded`, `failed`; agent responses use `completed`,
  `failed`, `cancelled`. Rationale: keeps validation deterministic while
  matching common audit lifecycle stages. Date/Author: 2026-01-28, Codex.

## Outcomes & Retrospective

Implementation delivered typed audit metadata for tool calls and agent
responses, validation for required fields, and round-trip coverage through
Postgres and in-memory repositories. Documentation now captures the schema
decision and roadmap item 1.1.2 is complete.

## Context and Orientation

Current code already implements the canonical message schema (`src/message/`),
in-memory and Postgres repositories, and migrations for `conversations`,
`messages`, `domain_events`, and `audit_logs`. Message metadata currently
captures agent backend, turn id, and slash command expansion. Postgres adapters
support audit context propagation via session settings and triggers.
Integration support for embedded Postgres exists under
`tests/postgres/cluster`, and integration tests already validate audit logs for
message inserts.

Key files and modules:

- `src/message/domain/message.rs` and `metadata.rs` for domain models.
- `src/message/ports/repository.rs` for the message persistence port.
- `src/message/adapters/postgres/` for Diesel-based persistence.
- `tests/postgres/` and `tests/in_memory/` for repository integration tests.
- `migrations/` for database schema and audit trigger definitions.

The gap for roadmap 1.1.2 is explicit audit metadata for tool calls and agent
responses within stored messages, plus a cohesive workflow for appending and
retrieving ordered message history by conversation id, backed by Postgres with
embedded test coverage.

## Plan of Work

### Stage A: Requirements Alignment and Metadata Schema

Review corbusier-design.md §2.2.1, §5.2.1, and §6.2.3 to define the minimal
shape for audit metadata. Decide on typed fields for tool call metadata and
agent response metadata, ensure they remain optional, and document the decision
in `docs/corbusier-design.md` under the relevant persistence or schema section.
If multiple interpretations remain, stop and request clarification.

### Stage B: Domain Model and Ports

Introduce typed audit metadata in the domain layer and align validation rules.
Likely changes include:

- Extend `src/message/domain/metadata.rs` with new structs such as
  `ToolCallAudit` and `AgentResponseAudit`, referenced from `MessageMetadata`.
- Update `MessageMetadata` constructors/builders and add helper methods for
  appending tool call and agent response audit metadata.
- Add validation logic in `src/message/validation` for required fields (e.g.
  tool call id, tool name, status) and for any new constraints.
- If needed, add a small application-style service in `src/message/history/`
  (new module) that orchestrates sequence allocation and message append, using
  only ports and domain types.

### Stage C: Adapters and Persistence

Update adapters to persist and retrieve the expanded metadata, and to expose
queryable conversation history with immutable ordering.

- Ensure `NewMessage::try_from_domain` and `row_to_message` round-trip the
  expanded metadata JSON without loss.
- If a conversation history service is added, implement it using existing
  ports (`MessageRepository`, optional new `ConversationRepository`) and keep
  adapters thin.
- If conversation existence is required, add a `ConversationRepository` port
  and implement in-memory and Postgres adapters using the existing
  `conversations` table and models (`ConversationRow`, `NewConversation`).
- Preserve audit context support in Postgres (`store_with_audit`) and ensure
  any new inserts (conversation creation, domain events) also apply audit
  context when needed.

### Stage D: Tests (Unit, Integration, Behavioural)

Add comprehensive tests covering happy and unhappy paths.

- **Unit tests (rstest)** in `src/message/tests/` for:
  - Metadata builders and validation failures (missing tool call id, missing
    tool name, invalid status, missing agent backend).
  - Message ordering semantics when appending via the history service.
- **Postgres integration tests** in `tests/postgres/` using the embedded
  cluster helper to verify:
  - Audit metadata persists and round-trips via JSONB.
  - Conversation history query returns ordered messages with metadata.
- **Behavioural tests (rstest-bdd v0.4.0)** under `tests/features/`:
  - Scenario: persist tool call + agent response messages and retrieve history
    with audit metadata.
  - Scenario: missing audit metadata yields validation error.
  Use `#[tokio::test]` on scenario functions to allow async repository calls.

### Stage E: Documentation, Roadmap, and Quality Gates

- Update `docs/users-guide.md` with user-visible behaviour (audit metadata now
  included in conversation history results).
- Record the audit metadata schema decision in `docs/corbusier-design.md`.
- Mark roadmap item 1.1.2 as done in `docs/roadmap.md`.
- Run formatting, linting, tests, markdown lint, and Mermaid validation.

## Concrete Steps

All commands run from repository root `/home/user/project`.

### Stage A

1. Read design references for audit metadata requirements:

   - `docs/corbusier-design.md` §2.2.1, §5.2.1, §6.2.1.2, §6.2.3
   - `docs/reliable-testing-in-rust-via-dependency-injection.md`
   - `docs/rust-testing-with-rstest-fixtures.md`
   - `docs/rstest-bdd-users-guide.md`

2. Document the audit metadata schema decision in
   `docs/corbusier-design.md`.

### Stage B

1. Add domain types and helpers in `src/message/domain/metadata.rs` (or new
   module if the file approaches 400 lines).
2. Update validation rules in `src/message/validation/` to enforce metadata
   invariants.
3. Add (if needed) a history service module under `src/message/history/` with
   a port trait for appending/retrieving conversation history.

### Stage C

1. Update adapters to round-trip new metadata fields and add any new
   repository implementations (in-memory and Postgres) required for
   conversation history.
2. Add or extend migration usage in tests only if required (avoid new
   migrations unless strictly necessary).

### Stage D

1. Unit tests (`rstest`) in `src/message/tests/` for new metadata types and
   validation.
2. Integration tests in `tests/postgres/` using the embedded cluster helper.
3. Behavioural tests:

   - Add `tests/features/conversation_history.feature`.
   - Implement step definitions using `rstest_bdd_macros` with
     `#[tokio::test]` on scenario functions.

### Stage E

1. Update `docs/users-guide.md` and wrap paragraphs at 80 columns.
2. Update `docs/corbusier-design.md` with the metadata schema decision.
3. Mark roadmap item 1.1.2 and its sub-items as done in `docs/roadmap.md`.
4. Run quality gates via Makefile targets, using `tee` for long output:

   - `set -o pipefail && make check-fmt 2>&1 | tee /tmp/check-fmt.log`
   - `set -o pipefail && make lint 2>&1 | tee /tmp/lint.log`
   - `set -o pipefail && make test 2>&1 | tee /tmp/test.log`
   - `set -o pipefail && make markdownlint 2>&1 | tee /tmp/markdownlint.log`
   - `set -o pipefail && make nixie 2>&1 | tee /tmp/nixie.log`

## Validation and Acceptance

Quality criteria (done means all of the following are true):

- Conversation history is queryable by conversation id and returned in
  sequence order.
- Tool call and agent response audit metadata round-trips through persistence
  and is visible in retrieved messages.
- `make check-fmt`, `make lint`, `make test`, `make markdownlint`, and
  `make nixie` all succeed.
- Behavioural tests (rstest-bdd) cover at least one happy path and one unhappy
  path, and all tests pass under embedded Postgres.

## Idempotence and Recovery

All steps are re-runnable. If a step fails:

1. Fix the reported issue in the referenced file.
2. Re-run the failed command from the Concrete Steps section.

No destructive actions are required. Use git to restore files if necessary.

## Artifacts and Notes

Expected files added or modified (names may adjust to fit line limits):

- `src/message/domain/metadata.rs` (extended audit metadata types)
- `src/message/validation/` (metadata validation updates)
- `src/message/history/mod.rs` and `src/message/history/service.rs` (if a
  dedicated history service is introduced)
- `src/message/ports/` (new conversation/history port if needed)
- `src/message/adapters/` (conversation repository implementations)
- `tests/features/conversation_history.feature`
- `tests/conversation_history_steps.rs`
- `tests/postgres/*` (new integration tests)
- `docs/corbusier-design.md`
- `docs/users-guide.md`
- `docs/roadmap.md`

## Interfaces and Dependencies

Expected additions (adjust if design review dictates otherwise):

- `ToolCallAudit` and `AgentResponseAudit` structs in
  `crate::message::domain::metadata`.
- Optional `ConversationRepository` or `ConversationHistoryPort` trait in
  `src/message/ports/` with methods such as:

  ```rust
  async fn ensure_conversation(
      &self,
      id: ConversationId,
  ) -> RepositoryResult<()>;
  async fn append_message(&self, message: Message, audit: AuditContext)
      -> RepositoryResult<()>;
  async fn history(&self, conversation_id: ConversationId)
      -> RepositoryResult<Vec<Message>>;
  ```

- Dev-dependencies:
  - `rstest-bdd = "0.4.0"`
  - `rstest-bdd-macros = { version = "0.4.0", features =
    ["strict-compile-time-validation"] }`

If dependency versions or feature flags differ from what is available, stop and
escalate before proceeding.
