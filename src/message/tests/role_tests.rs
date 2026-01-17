//! Unit tests for Role parsing and serialization.

use crate::message::domain::Role;
use rstest::rstest;

// ============================================================================
// Role::as_str tests
// ============================================================================

#[rstest]
#[case(Role::User, "user")]
#[case(Role::Assistant, "assistant")]
#[case(Role::Tool, "tool")]
#[case(Role::System, "system")]
fn role_as_str_returns_correct_string(#[case] role: Role, #[case] expected: &str) {
    assert_eq!(role.as_str(), expected);
}

// ============================================================================
// TryFrom<&str> for Role tests
// ============================================================================

#[rstest]
#[case("user", Role::User)]
#[case("assistant", Role::Assistant)]
#[case("tool", Role::Tool)]
#[case("system", Role::System)]
fn role_try_from_str_parses_valid_roles(#[case] input: &str, #[case] expected: Role) {
    let result = Role::try_from(input);
    assert_eq!(result, Ok(expected));
}

#[rstest]
#[case("")]
#[case("User")]
#[case("ASSISTANT")]
#[case("TOOL")]
#[case("SYSTEM")]
#[case("human")]
#[case("bot")]
#[case("ai")]
#[case("unknown")]
#[case("user ")]
#[case(" user")]
#[case("user\n")]
fn role_try_from_str_rejects_invalid_roles(#[case] input: &str) {
    let result = Role::try_from(input);
    assert!(result.is_err());
}

#[test]
fn role_parse_error_display_includes_invalid_input() {
    let result = Role::try_from("invalid_role");
    let err = result.expect_err("should fail for invalid input");
    let display = format!("{err}");
    assert!(display.contains("invalid_role"));
    assert!(display.contains("invalid role"));
}

// ============================================================================
// Round-trip conversion tests
// ============================================================================

#[rstest]
#[case(Role::User)]
#[case(Role::Assistant)]
#[case(Role::Tool)]
#[case(Role::System)]
fn role_round_trip_as_str_and_try_from(#[case] role: Role) {
    let string = role.as_str();
    let parsed = Role::try_from(string).expect("round-trip should succeed");
    assert_eq!(parsed, role);
}

// ============================================================================
// Display trait tests
// ============================================================================

#[rstest]
#[case(Role::User, "user")]
#[case(Role::Assistant, "assistant")]
#[case(Role::Tool, "tool")]
#[case(Role::System, "system")]
fn role_display_matches_as_str(#[case] role: Role, #[case] expected: &str) {
    assert_eq!(format!("{role}"), expected);
    assert_eq!(role.to_string(), expected);
}
