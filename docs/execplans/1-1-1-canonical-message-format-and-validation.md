# Implement Canonical Message Format and Validation (Roadmap 1.1.1)

This Execution Plan (ExecPlan) is a living document. The sections
`Constraints`, `Tolerances`, `Risks`, `Progress`, `Surprises & Discoveries`,
`Decision Log`, and `Outcomes & Retrospective` must be kept up to date as work
proceeds.

Status: COMPLETE

This document must be maintained in accordance with the execplans skill
documented at `.claude/skills/execplans/SKILL.md`.

## Purpose / Big Picture

After this change, Corbusier will have a type-safe canonical message format
that unifies messages from any agent backend (Claude Code SDK, Codex, etc.)
into a single schema. Users will be able to store and retrieve conversation
messages with guaranteed structure validation, immutability guarantees, and
schema versioning support for future migrations.

Observable success: unit tests pass demonstrating message creation, validation,
and rejection of malformed input; behavioural tests verify end-to-end scenarios
for user, assistant, and tool messages.

## Constraints

Hard invariants that must not be violated:

- **No panic-prone code**: The strict Clippy lints in `Cargo.toml` deny
  `unwrap_used`, `expect_used`, `indexing_slicing`, `panic_in_result_fn`. All
  code must return `Result` or use safe alternatives.
- **Hexagonal architecture**: Domain types must not import infrastructure
  concerns. The dependency rule (inward-pointing) must hold. See
  `.claude/skills/hexagonal-architecture/SKILL.md`.
- **Immutable messages**: Messages are append-only and cannot be modified after
  creation per corbusier-design.md F-001-RQ-002.
- **File size limit**: No source file may exceed 400 lines per `AGENTS.md`.
- **Documentation**: Every module requires `//!` doc comments; public APIs
  require `///` rustdoc.
- **en-GB-oxendict spelling**: Comments and docs use British spelling with
  Oxford -ize endings.

## Tolerances (Exception Triggers)

Thresholds that trigger escalation:

- **Scope**: If implementation requires more than 20 new files or 2000 lines of
  code (net), stop and escalate.
- **Dependencies**: If more than 8 new crate dependencies are required beyond
  those listed in this plan, stop and escalate.
- **Test iterations**: If any test fails after 3 fix attempts, stop and
  escalate.
- **Interface changes**: If the message format deviates materially from
  corbusier-design.md section 2.2.1 or 6.2.1.2, stop and escalate.
- **Performance**: If message validation exceeds 10ms in benchmarks (target is
  <2ms), stop and escalate.

## Risks

    - Risk: NewType boilerplate explosion
      Severity: medium
      Likelihood: medium
      Mitigation: Use newtype wrappers for homogeneous types; keep custom impls
      minimal. Monitor file sizes.

    - Risk: Serde configuration complexity for tagged enums
      Severity: low
      Likelihood: medium
      Mitigation: Write unit tests for each ContentPart variant early;
      verify round-trip serialisation.

    - Risk: Clock abstraction adds complexity
      Severity: low
      Likelihood: low
      Mitigation: Use mockable crate's Clock trait as documented in
      docs/reliable-testing-in-rust-via-dependency-injection.md.

    - Risk: rstest-bdd (Behaviour-Driven Development (BDD)) learning curve
      Severity: medium
      Likelihood: medium
      Mitigation: Start with simple scenarios; refer to
      docs/rstest-bdd-users-guide.md for patterns.

## Progress

    - [x] Stage A: Project setup and dependencies
    - [x] Stage B: Domain types implementation
    - [x] Stage C: Error types and ports
    - [x] Stage D: Validation service
    - [x] Stage E: Schema versioning
    - [x] Stage F: Unit tests
    - [x] Stage G: Behavioural tests (implemented as plain Rust integration
          tests)
    - [x] Stage H: Documentation and cleanup
    - [x] Stage I: Mark roadmap complete

## Surprises & Discoveries

- The mockable crate uses `utc()` not `now()` for getting the current UTC time.
- The rstest-bdd crate does not appear to be available in the public crates.io
  registry. Behavioural tests were later implemented using plain Rust
  integration tests in `tests/message_validation_integration.rs`.
- Clippy requires `#[expect]` instead of `#[allow]` with a `reason` parameter
  for lint suppressions in this project.
- The project enforces `Self` type aliases in enum variants (e.g.,
  `Multiple(Vec<Self>)` instead of `Multiple(Vec<ValidationError>)`).

## Decision Log

    - Decision: Use feature-based module layout (src/message/) rather than
      layer-based (src/domain/, src/ports/)
      Rationale: Aligns with AGENTS.md "group by feature, not layer" and
      hexagonal architecture skill guidance for smaller projects.
      Date/Author: Initial plan

    - Decision: Use mockable crate for Clock abstraction
      Rationale: Documented in docs/reliable-testing-in-rust-via-dependency-
      injection.md as the preferred approach; avoids environment mutation.
      Date/Author: Initial plan

    - Decision: Store ContentPart as serde tagged enum with snake_case
      Rationale: Matches the JSONB "type" field pattern in
      corbusier-design.md section 6.2.1.2.
      Date/Author: Initial plan

## Outcomes & Retrospective

**Completed successfully.** All quality gates pass:

- `make all` (check-fmt, lint, test) passes with 108 unit tests + 8 integration
  tests + 17 doctests
- No unsafe code; all inputs validated before processing
- Messages are immutable after creation (no setter methods)
- Schema versioning implemented with v1 to v2 upgrade path

**Deviations from plan:**

- Behavioural tests (Stage G) implemented as plain Rust integration tests
  instead of rstest-bdd (crate not available).
- File count is 20 files (vs 20 estimated) — integration tests added in
  `tests/message_validation_integration.rs`.

**Key metrics:**

- Test count: 108 unit tests, 8 integration tests, 17 doctests
- Files created: 20 source files (including integration test file)
- Dependencies added: 7 runtime, 2 dev-dependencies (within tolerance of 8)

## Context and Orientation

Corbusier is an AI agent orchestration platform at the earliest stage of
development. The existing codebase consists only of:

- `/root/repo/src/main.rs`: A stub printing "Hello from Corbusier!"
- `/root/repo/Cargo.toml`: Project configuration with strict Clippy lints
- `/root/repo/docs/corbusier-design.md`: Technical specification (~5600 lines)
- `/root/repo/docs/roadmap.md`: Phased delivery plan

The canonical message format is defined in corbusier-design.md sections 2.2.1
(Conversation Management Requirements) and 6.2.1.2 (Data Models and Structures).

Key terms:

- **Message**: An atomic unit of conversation history with id, role, content
  parts, metadata, timestamp, and sequence number.
- **Role**: One of User, Assistant, Tool, or System indicating the message
  source.
- **ContentPart**: A polymorphic content element (text, tool call, tool result,
  or attachment).
- **Port**: An abstract trait interface the domain exposes (driving) or
  requires (driven).
- **Adapter**: A concrete implementation of a port (e.g., PostgreSQL
  repository).

## Plan of Work

### Stage A: Project Setup and Dependencies

Add required dependencies to `Cargo.toml`. Create the module directory
structure. This stage produces no functional code but establishes the scaffold.

**Validation**: `cargo check` succeeds with no errors.

### Stage B: Domain Types Implementation

Implement domain types in order of dependency:

1. NewType identifiers (`MessageId`, `ConversationId`, `TurnId`,
   `SequenceNumber`) in `src/message/domain/ids.rs`
2. Role enum in `src/message/domain/role.rs`
3. ContentPart types (`TextPart`, `ToolCallPart`, `ToolResultPart`,
   `AttachmentPart`) in `src/message/domain/content.rs`
4. MessageMetadata in `src/message/domain/metadata.rs`
5. Message aggregate with builder in `src/message/domain/message.rs`

Each file includes module documentation and rustdoc for public items.

**Validation**: `cargo check` succeeds; `cargo doc --no-deps` generates
documentation.

### Stage C: Error Types and Ports

1. Define `ValidationError`, `RepositoryError`, `SchemaUpgradeError` in
   `src/message/error.rs` using thiserror
2. Define `MessageRepository` port trait in `src/message/ports/repository.rs`
3. Define `MessageValidator` port trait with `ValidationConfig` in
   `src/message/ports/validator.rs`

**Validation**: `cargo check` succeeds.

### Stage D: Validation Service

1. Implement validation rules as pure functions in
   `src/message/validation/rules.rs`
2. Implement `DefaultMessageValidator` in `src/message/validation/service.rs`

Validation rules to implement:

- Message ID non-nil check
- Content not empty check
- Content parts validation (text, tool call, attachment)
- Message size limit check

**Validation**: `cargo check` succeeds.

### Stage E: Schema Versioning

1. Define `VersionedEvent` and `EventMetadata` in
   `src/message/versioning/event.rs`
2. Define `EventUpgrader` trait and `MessageCreatedUpgrader` implementation in
   `src/message/versioning/upgrader.rs`
3. Implement `UpgraderRegistry` for event type dispatch

**Validation**: `cargo check` succeeds.

### Stage F: Unit Tests

Create `src/message/tests/` module with:

1. `domain_tests.rs`: Tests for MessageId, Role, ContentPart, Message builder
2. `validation_tests.rs`: Tests for DefaultMessageValidator covering happy
   paths, empty content, invalid tool calls, size limits
3. `versioning_tests.rs`: Tests for EventUpgrader v1 to v2 migration

Use rstest fixtures for shared setup (MockClock, valid messages).

**Validation**: `make test` passes with all new tests.

### Stage G: Behavioural Tests

Create `tests/features/message_validation.feature` with Gherkin scenarios:

- Valid user message accepted
- Empty content rejected
- Empty text rejected
- Invalid tool call rejected

Implement step definitions in `tests/message_validation_steps.rs`.

**Validation**: `make test` passes, including BDD tests.

### Stage H: Documentation and Cleanup

1. Update `docs/users-guide.md` with message format documentation
2. Add design decisions to `docs/corbusier-design.md` if any were made
3. Run `make fmt`, `make lint`, `make check-fmt`
4. Verify all tests pass

**Validation**: All quality gates pass.

### Stage I: Mark Roadmap Complete

Update `docs/roadmap.md` to mark item 1.1.1 as done:

    - [x] 1.1.1 Implement the canonical message format and validation.

**Validation**: Git diff shows only the checkbox change.

## Concrete Steps

All commands run from repository root `/root/repo`.

### Stage A Commands

1. Edit `Cargo.toml` to add dependencies:

       [dependencies]
       serde = { version = "1.0", features = ["derive"] }
       serde_json = "1.0"
       chrono = { version = "0.4", features = ["serde"] }
       uuid = { version = "1.0", features = ["v4", "serde"] }
       thiserror = "2.0"
       async-trait = "0.1"
       mockable = { version = "0.1", default-features = false, features = ["clock"] }

       [dev-dependencies]
       rstest = "0.24"
       mockall = "0.13"

2. Create directory structure:

       mkdir -p src/message/domain
       mkdir -p src/message/ports
       mkdir -p src/message/validation
       mkdir -p src/message/versioning
       mkdir -p src/message/tests

3. Create module files (touch to establish):

       touch src/message/mod.rs
       touch src/message/domain/mod.rs
       touch src/message/domain/ids.rs
       touch src/message/domain/role.rs
       touch src/message/domain/content.rs
       touch src/message/domain/metadata.rs
       touch src/message/domain/message.rs
       touch src/message/error.rs
       touch src/message/ports/mod.rs
       touch src/message/ports/repository.rs
       touch src/message/ports/validator.rs
       touch src/message/validation/mod.rs
       touch src/message/validation/rules.rs
       touch src/message/validation/service.rs
       touch src/message/versioning/mod.rs
       touch src/message/versioning/event.rs
       touch src/message/versioning/upgrader.rs
       touch src/message/tests/mod.rs

4. Verify:

       cargo check

   Expected: Compilation succeeds (empty modules with docs).

### Stage B-H Commands

Each stage follows the pattern:

1. Implement code per Plan of Work section
2. Run `cargo check` after each file
3. Run `make fmt` to format
4. Run `make lint` to verify Clippy
5. Run `make test` after tests are added

Final validation:

    set -o pipefail && make test 2>&1 | tee /tmp/test-output.log

Expected: All tests pass, exit code 0.

## Validation and Acceptance

**Quality criteria (what "done" means):**

- Tests: `make test` passes; new tests cover message creation, validation
  success, validation failure for each error variant
- Lint/typecheck: `make lint` and `make check-fmt` pass with no warnings
- Performance: Message validation completes in <2ms (verify via test timing)
- Security: No unsafe code; all inputs validated before processing

**Quality method (how we check):**

    make check-fmt && make lint && make test

All three commands must succeed with exit code 0.

**Behavioural acceptance:**

- A Message can be created with valid content and passes validation
- A Message with empty content is rejected with `ValidationError::EmptyContent`
- A Message with empty text (when disallowed) is rejected with
  `ValidationError::InvalidContentPart`
- A ToolCallPart without call_id is rejected
- Messages are immutable after creation (no setter methods)
- Schema versioning upgrades v1 events to v2

## Idempotence and Recovery

All steps are idempotent:

- Creating files that exist has no effect (touch)
- Cargo operations are safe to repeat
- Running tests multiple times produces consistent results

If a step fails:

1. Check the error message
2. Fix the issue in the relevant file
3. Re-run from the failed command

No destructive operations are performed; git can restore any file.

## Artefacts and Notes

### File Structure After Implementation

    src/
      lib.rs                          # Re-export message module
      main.rs                         # Application entry (unchanged)
      message/
        mod.rs                        # Feature module (~30 lines)
        error.rs                      # Error types (~120 lines)
        domain/
          mod.rs                      # Barrel export (~20 lines)
          ids.rs                      # NewType IDs (~100 lines)
          role.rs                     # Role enum (~50 lines)
          content.rs                  # ContentPart types (~180 lines)
          metadata.rs                 # MessageMetadata (~80 lines)
          message.rs                  # Message aggregate (~200 lines)
        ports/
          mod.rs                      # Barrel export (~10 lines)
          repository.rs              # Repository trait (~60 lines)
          validator.rs               # Validator trait (~70 lines)
        validation/
          mod.rs                      # Barrel export (~10 lines)
          rules.rs                    # Validation rules (~150 lines)
          service.rs                  # DefaultMessageValidator (~80 lines)
        versioning/
          mod.rs                      # Barrel export (~10 lines)
          event.rs                    # VersionedEvent (~60 lines)
          upgrader.rs                 # EventUpgrader trait (~100 lines)
        tests/
          mod.rs                      # Test module (~10 lines)
          domain_tests.rs            # Domain unit tests (~150 lines)
          validation_tests.rs        # Validation tests (~120 lines)
          versioning_tests.rs        # Versioning tests (~80 lines)
    tests/
      features/
        message_validation.feature   # BDD scenarios (~40 lines)
      message_validation_steps.rs    # Step definitions (~150 lines)

Estimated total: ~1700 lines across ~20 files.

## Interfaces and Dependencies

### Crate Dependencies

    serde = "1.0"          # Serialisation framework
    serde_json = "1.0"     # JSON support
    chrono = "0.4"         # Date/time handling
    uuid = "1.0"           # UUID generation
    thiserror = "2.0"      # Error derive macro
    async-trait = "0.1"    # Async trait support
    mockable = "0.1"       # Clock abstraction

    [dev-dependencies]
    rstest = "0.24"        # Test fixtures
    mockall = "0.13"       # Mock generation

### Key Type Signatures

In `src/message/domain/ids.rs`:

    pub struct MessageId(Uuid);
    pub struct ConversationId(Uuid);
    pub struct TurnId(Uuid);
    pub struct SequenceNumber(u64);

In `src/message/domain/role.rs`:

    pub enum Role { User, Assistant, Tool, System }

In `src/message/domain/content.rs`:

    pub enum ContentPart {
        Text(TextPart),
        ToolCall(ToolCallPart),
        ToolResult(ToolResultPart),
        Attachment(AttachmentPart),
    }

In `src/message/domain/message.rs`:

    pub struct Message { … }
    impl Message {
        pub fn new(…, clock: &impl Clock) -> Result<Self, MessageBuilderError>;
        pub fn builder(…) -> MessageBuilder;
    }

In `src/message/ports/repository.rs`:

    #[async_trait]
    pub trait MessageRepository: Send + Sync {
        async fn store(&self, message: &Message) -> RepositoryResult<()>;
        async fn find_by_id(&self, id: MessageId) -> RepositoryResult<Option<Message>>;
        async fn find_by_conversation(&self, id: ConversationId) -> RepositoryResult<Vec<Message>>;
    }

In `src/message/ports/validator.rs`:

    pub trait MessageValidator: Send + Sync {
        fn validate(&self, message: &Message) -> ValidationResult<()>;
    }

In `src/message/versioning/upgrader.rs`:

    pub trait EventUpgrader: Send + Sync {
        fn upgrade(&self, event: VersionedEvent) -> UpgradeResult<VersionedEvent>;
        fn current_version(&self) -> u32;
        fn supports_version(&self, version: u32) -> bool;
    }
