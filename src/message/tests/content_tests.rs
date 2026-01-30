//! Unit tests for content part types and metadata.

#![expect(
    clippy::too_many_arguments,
    reason = "rstest case expansion creates many parameters from #[case] attributes"
)]

use crate::message::domain::{
    AgentResponseAudit, AgentResponseStatus, AttachmentPart, MessageMetadata, Role, TextPart,
    ToolCallAudit, ToolCallPart, ToolCallStatus, ToolResultPart, TurnId,
};
use rstest::rstest;
use serde_json::json;

// ============================================================================
// Role tests
// ============================================================================

#[rstest]
#[case(Role::User, false, true, false, false)]
#[case(Role::Assistant, true, false, false, false)]
#[case(Role::Tool, false, false, false, true)]
#[case(Role::System, false, false, true, false)]
fn role_capabilities(
    #[case] role: Role,
    #[case] can_call_tools: bool,
    #[case] is_human: bool,
    #[case] is_system: bool,
    #[case] is_tool: bool,
) {
    assert_eq!(role.can_call_tools(), can_call_tools);
    assert_eq!(role.is_human(), is_human);
    assert_eq!(role.is_system(), is_system);
    assert_eq!(role.is_tool(), is_tool);
}

#[rstest]
#[case(Role::User, "user")]
#[case(Role::Assistant, "assistant")]
#[case(Role::Tool, "tool")]
#[case(Role::System, "system")]
fn role_display(#[case] role: Role, #[case] expected: &str) {
    assert_eq!(role.to_string(), expected);
}

#[rstest]
#[case(Role::User)]
#[case(Role::Assistant)]
#[case(Role::Tool)]
#[case(Role::System)]
fn role_serialization_round_trip(#[case] role: Role) {
    let json = serde_json::to_string(&role).expect("serialize");
    let deserialized: Role = serde_json::from_str(&json).expect("deserialize");
    assert_eq!(role, deserialized);
}

// ============================================================================
// TextPart tests
// ============================================================================

#[rstest]
fn text_part_new() {
    let text = TextPart::new("Hello, world!");
    assert_eq!(text.text, "Hello, world!");
}

#[rstest]
#[case("", true)]
#[case("   ", true)]
#[case("\n\t", true)]
#[case("hello", false)]
#[case(" hello ", false)]
fn text_part_is_empty(#[case] content: &str, #[case] expected: bool) {
    let text = TextPart::new(content);
    assert_eq!(text.is_empty(), expected);
}

#[rstest]
fn text_part_len() {
    let text = TextPart::new("hello");
    assert_eq!(text.len(), 5);
}

// ============================================================================
// ToolCallPart tests
// ============================================================================

#[rstest]
fn tool_call_part_new() {
    let call = ToolCallPart::new("call-123", "my_tool", json!({"arg": "value"}));
    assert_eq!(call.call_id, "call-123");
    assert_eq!(call.name, "my_tool");
    assert_eq!(call.arguments, json!({"arg": "value"}));
}

#[rstest]
#[case("call-123", "tool", true)]
#[case("", "tool", false)]
#[case("call-123", "", false)]
#[case("", "", false)]
fn tool_call_is_valid(#[case] call_id: &str, #[case] name: &str, #[case] expected: bool) {
    let call = ToolCallPart::new(call_id, name, json!({}));
    assert_eq!(call.is_valid(), expected);
}

// ============================================================================
// ToolResultPart tests
// ============================================================================

#[rstest]
fn tool_result_success() {
    let result = ToolResultPart::success("call-123", json!({"data": "result"}));
    assert!(result.success);
    assert_eq!(result.call_id, "call-123");
}

#[rstest]
fn tool_result_failure() {
    let result = ToolResultPart::failure("call-123", "Something went wrong");
    assert!(!result.success);
    assert_eq!(result.content, json!("Something went wrong"));
}

#[rstest]
fn tool_result_is_valid() {
    let valid = ToolResultPart::success("call-123", json!({}));
    let invalid = ToolResultPart::success("", json!({}));
    assert!(valid.is_valid());
    assert!(!invalid.is_valid());
}

// ============================================================================
// AttachmentPart tests
// ============================================================================

#[rstest]
fn attachment_part_new() {
    let attachment = AttachmentPart::new("text/plain", "SGVsbG8=");
    assert_eq!(attachment.mime_type, "text/plain");
    assert_eq!(attachment.data, "SGVsbG8=");
    assert!(attachment.name.is_none());
    assert!(attachment.size_bytes.is_none());
}

#[rstest]
fn attachment_part_with_name_and_size() {
    let attachment = AttachmentPart::new("image/png", "data")
        .with_name("image.png")
        .with_size(1024);
    assert_eq!(attachment.name, Some("image.png".to_owned()));
    assert_eq!(attachment.size_bytes, Some(1024));
}

#[rstest]
#[case("text/plain", "data", true)]
#[case("", "data", false)]
#[case("text/plain", "", false)]
fn attachment_is_valid(#[case] mime_type: &str, #[case] data: &str, #[case] expected: bool) {
    let attachment = AttachmentPart::new(mime_type, data);
    assert_eq!(attachment.is_valid(), expected);
}

// ============================================================================
// MessageMetadata tests
// ============================================================================

#[rstest]
fn message_metadata_empty() {
    let metadata = MessageMetadata::empty();
    assert!(metadata.is_empty());
}

#[rstest]
fn message_metadata_with_agent_backend() {
    let metadata = MessageMetadata::with_agent_backend("claude_code_sdk");
    assert_eq!(metadata.agent_backend, Some("claude_code_sdk".to_owned()));
    assert!(!metadata.is_empty());
}

#[rstest]
fn message_metadata_builder_chain() {
    let turn_id = TurnId::new();
    let tool_call = ToolCallAudit::new("call-1", "search", ToolCallStatus::Succeeded);
    let response = AgentResponseAudit::new(AgentResponseStatus::Completed).with_response_id("r-1");
    let metadata = MessageMetadata::with_agent_backend("claude")
        .with_turn_id(turn_id)
        .with_tool_call_audit(tool_call)
        .with_agent_response_audit(response)
        .with_extension("custom", json!({"key": "value"}));

    assert_eq!(metadata.agent_backend, Some("claude".to_owned()));
    assert_eq!(metadata.turn_id, Some(turn_id));
    assert_eq!(metadata.tool_call_audits.len(), 1);
    assert!(metadata.agent_response_audit.is_some());
    assert!(metadata.extensions.contains_key("custom"));
}

// ============================================================================
// Audit metadata tests
// ============================================================================

#[rstest]
fn tool_call_audit_new_sets_fields() {
    let audit = ToolCallAudit::new("call-123", "read_file", ToolCallStatus::Running);
    assert_eq!(audit.call_id, "call-123");
    assert_eq!(audit.tool_name, "read_file");
    assert_eq!(audit.status, ToolCallStatus::Running);
    assert!(audit.error.is_none());
}

#[rstest]
fn tool_call_audit_with_error() {
    let audit = ToolCallAudit::new("call-123", "read_file", ToolCallStatus::Failed)
        .with_error("permission denied");
    assert_eq!(audit.error, Some("permission denied".to_owned()));
}

#[rstest]
fn agent_response_audit_builders() {
    let audit = AgentResponseAudit::new(AgentResponseStatus::Completed)
        .with_response_id("resp-1")
        .with_model("claude-3-opus")
        .with_error("none");
    assert_eq!(audit.status, AgentResponseStatus::Completed);
    assert_eq!(audit.response_id, Some("resp-1".to_owned()));
    assert_eq!(audit.model, Some("claude-3-opus".to_owned()));
    assert_eq!(audit.error, Some("none".to_owned()));
}
