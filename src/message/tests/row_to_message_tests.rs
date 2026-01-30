//! Tests for `MessageRow` to domain `Message` conversion via `row_to_message`.
//!
//! Covers role parsing, content deserialization, sequence number handling,
//! timestamp preservation, and error cases for malformed data.

use crate::message::{
    adapters::models::MessageRow,
    adapters::postgres::row_to_message,
    domain::{AgentResponseStatus, Role, ToolCallStatus},
    error::RepositoryError,
};
use chrono::Utc;
use rstest::{fixture, rstest};
use serde_json::json;
use uuid::Uuid;

/// Provides a valid [`MessageRow`] for testing row-to-domain conversions.
///
/// Tests can override individual fields using struct update syntax:
/// `MessageRow { role: "assistant".to_owned(), ..message_row() }`.
#[fixture]
fn message_row() -> MessageRow {
    MessageRow {
        id: Uuid::new_v4(),
        conversation_id: Uuid::new_v4(),
        role: "user".to_owned(),
        content: json!([{"type": "text", "text": "Hello world"}]),
        metadata: json!({}),
        created_at: Utc::now(),
        sequence_number: 1,
    }
}

#[rstest]
fn row_to_message_converts_valid_row(message_row: MessageRow) {
    let expected_id = message_row.id;
    let expected_conv_id = message_row.conversation_id;

    let result = row_to_message(message_row);

    assert!(result.is_ok());
    let message = result.expect("conversion should succeed");
    assert_eq!(message.id().into_inner(), expected_id);
    assert_eq!(message.conversation_id().into_inner(), expected_conv_id);
    assert_eq!(message.role(), Role::User);
    assert_eq!(message.sequence_number().value(), 1);
}

#[rstest]
#[case("user", Role::User)]
#[case("assistant", Role::Assistant)]
#[case("tool", Role::Tool)]
#[case("system", Role::System)]
fn row_to_message_parses_all_role_variants(
    message_row: MessageRow,
    #[case] role_str: &str,
    #[case] expected_role: Role,
) {
    let row = MessageRow {
        role: role_str.to_owned(),
        ..message_row
    };

    let result = row_to_message(row);

    assert!(result.is_ok());
    assert_eq!(
        result.expect("conversion should succeed").role(),
        expected_role
    );
}

#[rstest]
fn row_to_message_fails_for_invalid_role(message_row: MessageRow) {
    let row = MessageRow {
        role: "invalid_role".to_owned(),
        ..message_row
    };

    let result = row_to_message(row);

    assert!(result.is_err());
    match result.expect_err("should fail for invalid role") {
        RepositoryError::Serialization(msg) => {
            assert!(
                msg.contains("invalid_role"),
                "error should mention role: {msg}"
            );
        }
        other => panic!("expected Serialization error, got {other:?}"),
    }
}

#[rstest]
fn row_to_message_fails_for_empty_content(message_row: MessageRow) {
    let row = MessageRow {
        content: json!([]),
        ..message_row
    };

    let result = row_to_message(row);

    assert!(result.is_err());
    match result.expect_err("should fail for empty content") {
        RepositoryError::Serialization(msg) => {
            assert!(
                msg.contains("empty") || msg.contains("content"),
                "error should mention empty content: {msg}"
            );
        }
        other => panic!("expected Serialization error, got {other:?}"),
    }
}

#[rstest]
fn row_to_message_fails_for_malformed_content_json(message_row: MessageRow) {
    let row = MessageRow {
        content: json!("not an array"),
        ..message_row
    };

    let result = row_to_message(row);

    assert!(result.is_err());
    match result.expect_err("should fail for malformed JSON") {
        RepositoryError::Serialization(_) => {}
        other => panic!("expected Serialization error, got {other:?}"),
    }
}

#[rstest]
fn row_to_message_fails_for_negative_sequence_number(message_row: MessageRow) {
    let row = MessageRow {
        sequence_number: -1,
        ..message_row
    };

    let result = row_to_message(row);

    assert!(result.is_err());
    match result.expect_err("should fail for negative sequence") {
        RepositoryError::Serialization(msg) => {
            assert!(
                msg.contains("out of range") || msg.contains("negative"),
                "error should mention range: {msg}"
            );
        }
        other => panic!("expected Serialization error, got {other:?}"),
    }
}

#[rstest]
fn row_to_message_handles_max_valid_sequence_number(message_row: MessageRow) {
    let row = MessageRow {
        sequence_number: i64::MAX,
        ..message_row
    };

    let result = row_to_message(row);

    assert!(result.is_ok());
    let message = result.expect("conversion should succeed");
    let expected_value = u64::try_from(i64::MAX).expect("i64::MAX should fit in u64");
    assert_eq!(message.sequence_number().value(), expected_value);
}

#[rstest]
fn row_to_message_preserves_timestamp(message_row: MessageRow) {
    let timestamp = Utc::now();
    let row = MessageRow {
        created_at: timestamp,
        ..message_row
    };

    let result = row_to_message(row);

    assert!(result.is_ok());
    let message = result.expect("conversion should succeed");
    assert_eq!(message.created_at(), timestamp);
}

#[rstest]
fn row_to_message_deserializes_complex_content(message_row: MessageRow) {
    let row = MessageRow {
        content: json!([
            {"type": "text", "text": "Hello"},
            {"type": "tool_call", "call_id": "call_123", "name": "search", "arguments": {"q": "test"}}
        ]),
        ..message_row
    };

    let result = row_to_message(row);

    assert!(result.is_ok());
    let message = result.expect("conversion should succeed");
    assert_eq!(message.content().len(), 2);
}

#[rstest]
fn row_to_message_deserializes_metadata_with_agent_backend(message_row: MessageRow) {
    let row = MessageRow {
        metadata: json!({"agent_backend": "claude-3-opus"}),
        ..message_row
    };

    let result = row_to_message(row);

    assert!(result.is_ok());
    let message = result.expect("conversion should succeed");
    assert_eq!(
        message.metadata().agent_backend,
        Some("claude-3-opus".to_owned())
    );
}

#[rstest]
fn row_to_message_deserializes_audit_metadata(message_row: MessageRow) {
    let row = MessageRow {
        metadata: json!({
            "tool_call_audits": [
                {
                    "call_id": "call-1",
                    "tool_name": "search",
                    "status": "succeeded",
                    "error": null
                }
            ],
            "agent_response_audit": {
                "status": "completed",
                "response_id": "resp-1",
                "model": "claude-3-opus"
            }
        }),
        ..message_row
    };

    let result = row_to_message(row);

    assert!(result.is_ok());
    let message = result.expect("conversion should succeed");
    assert_eq!(message.metadata().tool_call_audits.len(), 1);
    let tool_audit = message
        .metadata()
        .tool_call_audits
        .first()
        .expect("tool call audit should exist");
    assert_eq!(tool_audit.call_id, "call-1");
    assert_eq!(tool_audit.tool_name, "search");
    assert_eq!(tool_audit.status, ToolCallStatus::Succeeded);
    let response = message
        .metadata()
        .agent_response_audit
        .as_ref()
        .expect("response audit should exist");
    assert_eq!(response.status, AgentResponseStatus::Completed);
    assert_eq!(response.response_id.as_deref(), Some("resp-1"));
    assert_eq!(response.model.as_deref(), Some("claude-3-opus"));
}
