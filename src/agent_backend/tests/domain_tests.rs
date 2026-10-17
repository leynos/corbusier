//! Unit tests for agent backend domain types.

use crate::agent_backend::domain::{
    AgentBackendRegistration, AgentCapabilities, BackendDomainError, BackendInfo, BackendName,
    BackendStatus, ParseBackendStatusError, ToolCallRequest, TurnDomainError,
    deterministic_tool_call_id,
};
use mockable::DefaultClock;
use rstest::rstest;
use serde_json::{Value, json};

/// Helper to create a test registration with sensible defaults.
fn create_test_registration(
    raw_name: &str,
    supports_streaming: bool,
    supports_tool_calls: bool,
) -> Result<AgentBackendRegistration, BackendDomainError> {
    let clock = DefaultClock;
    let name = BackendName::new(raw_name)?;
    let capabilities = AgentCapabilities::new(supports_streaming, supports_tool_calls);
    let info = BackendInfo::new("Test Backend", "1.0.0", "Test")?;
    Ok(AgentBackendRegistration::new(
        name,
        capabilities,
        info,
        &clock,
    ))
}

// ── BackendName validation ─────────────────────────────────────────

#[rstest]
#[case("claude_code_sdk")]
#[case("codex_cli")]
#[case("backend_v2")]
#[case("a")]
fn valid_backend_names_are_accepted(#[case] input: &str) {
    let name = BackendName::new(input);
    assert!(name.is_ok(), "expected '{input}' to be valid");
    assert_eq!(name.expect("valid name").as_str(), input);
}

#[rstest]
fn backend_name_is_trimmed_and_lowercased() {
    let name = BackendName::new("  Claude_Code  ").expect("should accept after trim+lowercase");
    assert_eq!(name.as_str(), "claude_code");
}

#[rstest]
#[case("")]
#[case("   ")]
fn empty_or_whitespace_backend_name_is_rejected(#[case] input: &str) {
    let result = BackendName::new(input);
    assert!(matches!(result, Err(BackendDomainError::EmptyBackendName)));
}

#[rstest]
#[case("claude-code")]
#[case("backend.v2")]
#[case("backend/v2")]
#[case("backend v2")]
fn invalid_characters_in_backend_name_rejected(#[case] input: &str) {
    let result = BackendName::new(input);
    assert!(matches!(
        result,
        Err(BackendDomainError::InvalidBackendName(_))
    ));
}

#[rstest]
#[case(100, true)]
#[case(101, false)]
fn backend_name_length_boundary(#[case] length: usize, #[case] expected_ok: bool) {
    let name = "a".repeat(length);
    let result = BackendName::new(&name);
    if expected_ok {
        assert!(result.is_ok(), "expected length {length} to be accepted");
    } else {
        assert!(
            matches!(result, Err(BackendDomainError::BackendNameTooLong(_))),
            "expected length {length} to be rejected"
        );
    }
}

#[rstest]
#[case("  ".to_owned() + &"a".repeat(100) + "  ", true,
    "padded input whose normalized length is exactly 100 should be accepted")]
#[case("  ".to_owned() + &"a".repeat(101) + "  ", false,
    "padded input whose normalized length is 101 should be rejected")]
#[case("A".repeat(100), true,
    "uppercase input whose normalized length is exactly 100 should be accepted")]
#[case("A".repeat(101), false,
    "uppercase input whose normalized length is 101 should be rejected")]
fn backend_name_normalized_length_boundary(
    #[case] input: String,
    #[case] expected_ok: bool,
    #[case] label: &str,
) {
    let result = BackendName::new(&input);
    if expected_ok {
        assert!(result.is_ok(), "{label}");
    } else {
        assert!(
            matches!(result, Err(BackendDomainError::BackendNameTooLong(_))),
            "{label}"
        );
    }
}

// ── BackendStatus round-trip ───────────────────────────────────────

#[rstest]
#[case(BackendStatus::Active, "active")]
#[case(BackendStatus::Inactive, "inactive")]
fn backend_status_as_str_round_trip(#[case] status: BackendStatus, #[case] expected: &str) {
    assert_eq!(status.as_str(), expected);
    let parsed = BackendStatus::try_from(expected).expect("should parse");
    assert_eq!(parsed, status);
}

#[rstest]
fn unknown_backend_status_is_rejected() {
    let result = BackendStatus::try_from("unknown");
    assert!(matches!(result, Err(ParseBackendStatusError(_))));
}

// ── BackendInfo validation ─────────────────────────────────────────

#[rstest]
fn valid_backend_info_is_accepted() {
    let result = BackendInfo::new("Claude Code SDK", "1.0.0", "Anthropic");
    assert!(result.is_ok());
    let info = result.expect("valid info");
    assert_eq!(info.display_name(), "Claude Code SDK");
    assert_eq!(info.version(), "1.0.0");
    assert_eq!(info.provider(), "Anthropic");
}

#[rstest]
#[case("", "1.0.0", "Anthropic", BackendDomainError::EmptyDisplayName)]
#[case("SDK", "", "Anthropic", BackendDomainError::EmptyVersion)]
#[case("SDK", "1.0.0", "", BackendDomainError::EmptyProvider)]
fn empty_backend_info_field_is_rejected(
    #[case] display_name: &str,
    #[case] version: &str,
    #[case] provider: &str,
    #[case] expected: BackendDomainError,
) {
    let result = BackendInfo::new(display_name, version, provider);
    assert_eq!(result, Err(expected));
}

// ── AgentBackendRegistration construction ──────────────────────────

#[rstest]
fn new_registration_defaults_to_active() {
    let registration =
        create_test_registration("test_backend", true, true).expect("valid registration");

    assert_eq!(registration.status(), BackendStatus::Active);
    assert_eq!(registration.name().as_str(), "test_backend");
    assert_eq!(registration.created_at(), registration.updated_at());
}

#[rstest]
fn deactivate_changes_status_to_inactive() {
    let clock = DefaultClock;
    let mut registration =
        create_test_registration("test_backend", true, false).expect("valid registration");
    registration.deactivate(&clock);

    assert_eq!(registration.status(), BackendStatus::Inactive);
}

#[rstest]
fn activate_changes_status_to_active() {
    let clock = DefaultClock;
    let mut registration =
        create_test_registration("test_backend", true, false).expect("valid registration");
    registration.deactivate(&clock);
    registration.activate(&clock);

    assert_eq!(registration.status(), BackendStatus::Active);
}

#[rstest]
fn update_capabilities_replaces_capabilities() {
    let clock = DefaultClock;
    let name = BackendName::new("test_backend").expect("valid name");
    let capabilities = AgentCapabilities::new(true, false);
    let info = BackendInfo::new("Test", "1.0.0", "Test").expect("valid info");

    let mut registration = AgentBackendRegistration::new(name, capabilities, info, &clock);
    assert!(!registration.capabilities().supports_tool_calls());

    let new_capabilities = AgentCapabilities::new(true, true);
    registration.update_capabilities(new_capabilities, &clock);

    assert!(registration.capabilities().supports_tool_calls());
}

// ── AgentCapabilities builder ──────────────────────────────────────

#[rstest]
fn capabilities_builder_methods_work() {
    let caps = AgentCapabilities::new(true, true)
        .with_content_types(vec!["text".to_owned(), "image".to_owned()])
        .with_max_context_window(200_000);

    assert!(caps.supports_streaming());
    assert!(caps.supports_tool_calls());
    assert_eq!(caps.supported_content_types().len(), 2);
    assert_eq!(caps.max_context_window(), Some(200_000));
}

#[rstest]
fn capabilities_defaults_have_empty_content_types_and_no_window() {
    let caps = AgentCapabilities::new(false, false);

    assert!(!caps.supports_streaming());
    assert!(!caps.supports_tool_calls());
    assert!(caps.supported_content_types().is_empty());
    assert!(caps.max_context_window().is_none());
}

// ── Deterministic tool-call identifiers ────────────────────────────

/// Pins the identifier for a fixed tool call to a digest computed outside this
/// crate (`sha256sum` over the canonical payload). Call IDs are persisted and
/// compared across turns, so any change to hashing or hex rendering — such as
/// the move off `sha2`'s `{:x}` formatting — must not shift them.
#[rstest]
#[expect(
    clippy::panic_in_result_fn,
    reason = "Test uses assertions for verification while returning Result for error propagation"
)]
fn call_id_matches_externally_computed_digest() -> Result<(), TurnDomainError> {
    let tool_call = ToolCallRequest::new("search", json!({"query": "rust"}))?;

    assert_eq!(
        deterministic_tool_call_id(&tool_call, 0),
        "call-c58e8f52a1330a1f7f8da5a36babb52e7f8a9065ec286f74d73f68dc6194fcb9",
    );
    Ok(())
}

/// The identifier must always carry a full SHA-256 rendered as 64 lowercase
/// hex digits, whatever the payload; a shorter rendering would signal a
/// dropped leading zero.
/// Asserts `call_id` is the `call-` prefix plus a full SHA-256 rendered as 64
/// lowercase hex digits.
///
/// A missing prefix degrades to an empty digest so the length assertion
/// reports it, keeping the helper free of panicking accessors. The assertions
/// live here rather than in the caller so the parameterized test can propagate
/// construction errors with `?` without also tripping
/// `clippy::panic_in_result_fn`, whose expectation does not reach the functions
/// `rstest` generates per case.
fn assert_call_id_is_full_lowercase_hex(call_id: &str) {
    let digest = call_id.strip_prefix("call-").unwrap_or_default();
    assert_eq!(digest.len(), 64, "expected a full SHA-256 in {call_id}");
    assert!(
        digest
            .bytes()
            .all(|c| c.is_ascii_digit() || (b'a'..=b'f').contains(&c)),
        "expected lowercase hex digits in {call_id}",
    );
}

#[rstest]
#[case::empty_parameters("read_file", json!({}), 0)]
#[case::nested_parameters("write", json!({"b": [1, {"a": null}], "a": "x"}), 7)]
#[case::unicode_parameters("echo", json!({"text": "café ☕"}), 3)]
fn call_id_is_prefixed_full_length_lowercase_hex(
    #[case] tool_name: &str,
    #[case] parameters: Value,
    #[case] index: usize,
) -> Result<(), TurnDomainError> {
    let tool_call = ToolCallRequest::new(tool_name, parameters)?;

    assert_call_id_is_full_lowercase_hex(&deterministic_tool_call_id(&tool_call, index));
    Ok(())
}

/// Distinct call sites within a turn must not collide, so the index has to
/// feed the digest rather than merely decorate the prefix.
#[rstest]
#[expect(
    clippy::panic_in_result_fn,
    reason = "Test uses assertions for verification while returning Result for error propagation"
)]
fn call_id_varies_with_index() -> Result<(), TurnDomainError> {
    let tool_call = ToolCallRequest::new("search", json!({"query": "rust"}))?;

    assert_ne!(
        deterministic_tool_call_id(&tool_call, 0),
        deterministic_tool_call_id(&tool_call, 1),
    );
    Ok(())
}
