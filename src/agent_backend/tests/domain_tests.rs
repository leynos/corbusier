//! Unit tests for agent backend domain types.

use crate::agent_backend::domain::{
    AgentBackendRegistration, AgentCapabilities, BackendDomainError, BackendInfo, BackendName,
    BackendStatus, ParseBackendStatusError,
};
use mockable::DefaultClock;
use rstest::rstest;

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
