//! Behavioural integration tests for message creation and validation.
//!
//! These tests exercise end-to-end scenarios for message handling,
//! verifying that the complete flow from message creation through
//! validation works correctly.

use corbusier::message::{
    domain::{
        AttachmentPart, ContentPart, ConversationId, Message, Role, SequenceNumber, TextPart,
        ToolCallPart, ToolResultPart,
    },
    error::ValidationError,
    ports::validator::MessageValidator,
    validation::service::DefaultMessageValidator,
};
use mockable::DefaultClock;
use serde_json::json;

// ============================================================================
// Scenario: Valid user message is accepted
// ============================================================================

/// When a user submits a message with valid text content,
/// the system should accept and validate it successfully.
#[test]
fn valid_user_message_is_accepted() {
    // Arrange
    let clock = DefaultClock;
    let validator = DefaultMessageValidator::new();

    // Act
    let message = Message::new(
        ConversationId::new(),
        Role::User,
        vec![ContentPart::Text(TextPart::new(
            "Hello, I need help with my code.",
        ))],
        SequenceNumber::new(1),
        &clock,
    )
    .expect("message creation should succeed");

    let result = validator.validate(&message);

    // Assert
    assert!(result.is_ok(), "Valid user message should pass validation");
}

// ============================================================================
// Scenario: Valid assistant message with tool calls
// ============================================================================

/// When an assistant responds with both text and tool calls,
/// the system should validate the complete message structure.
#[test]
fn assistant_message_with_tool_calls_is_accepted() {
    // Arrange
    let clock = DefaultClock;
    let validator = DefaultMessageValidator::new();

    // Act
    let message = Message::new(
        ConversationId::new(),
        Role::Assistant,
        vec![
            ContentPart::Text(TextPart::new("Let me read that file for you.")),
            ContentPart::ToolCall(ToolCallPart::new(
                "call-12345",
                "read_file",
                json!({"path": "/src/main.rs"}),
            )),
        ],
        SequenceNumber::new(2),
        &clock,
    )
    .expect("message creation should succeed");

    let result = validator.validate(&message);

    // Assert
    assert!(
        result.is_ok(),
        "Assistant message with tool calls should pass validation"
    );
}

// ============================================================================
// Scenario: Tool response message is valid
// ============================================================================

/// When a tool returns results for a previous tool call,
/// the system should validate the tool result structure.
#[test]
fn tool_response_message_is_accepted() {
    // Arrange
    let clock = DefaultClock;
    let validator = DefaultMessageValidator::new();

    // Act
    let message = Message::new(
        ConversationId::new(),
        Role::Tool,
        vec![ContentPart::ToolResult(ToolResultPart::success(
            "call-12345",
            json!({"content": "fn main() { println!(\"Hello\"); }"}),
        ))],
        SequenceNumber::new(3),
        &clock,
    )
    .expect("message creation should succeed");

    let result = validator.validate(&message);

    // Assert
    assert!(
        result.is_ok(),
        "Tool response message should pass validation"
    );
}

// ============================================================================
// Scenario: Empty content is rejected
// ============================================================================

/// When a message is created without any content parts,
/// the system should reject it with an appropriate error.
#[test]
fn empty_content_is_rejected() {
    // Arrange
    let clock = DefaultClock;

    // Act
    let result = Message::new(
        ConversationId::new(),
        Role::User,
        vec![], // Empty content
        SequenceNumber::new(1),
        &clock,
    );

    // Assert
    assert!(
        result.is_err(),
        "Empty content should be rejected at message creation"
    );
}

// ============================================================================
// Scenario: Empty text content is rejected with default config
// ============================================================================

/// When text content contains only whitespace,
/// the default validator should reject it.
#[test]
fn whitespace_only_text_is_rejected() {
    // Arrange
    let clock = DefaultClock;
    let validator = DefaultMessageValidator::new();

    let message = Message::new(
        ConversationId::new(),
        Role::User,
        vec![ContentPart::Text(TextPart::new("   \n\t  "))],
        SequenceNumber::new(1),
        &clock,
    )
    .expect("message creation should succeed");

    // Act
    let result = validator.validate(&message);

    // Assert
    assert!(
        result.is_err(),
        "Whitespace-only text should be rejected by default"
    );
}

// ============================================================================
// Scenario: Invalid tool call is rejected
// ============================================================================

/// When a tool call is missing required fields,
/// the validator should reject the message.
#[test]
fn invalid_tool_call_is_rejected() {
    // Arrange
    let clock = DefaultClock;
    let validator = DefaultMessageValidator::new();

    let message = Message::new(
        ConversationId::new(),
        Role::Assistant,
        vec![ContentPart::ToolCall(ToolCallPart::new(
            "",          // Missing call_id
            "read_file", // Valid name
            json!({}),
        ))],
        SequenceNumber::new(1),
        &clock,
    )
    .expect("message creation should succeed");

    // Act
    let result = validator.validate(&message);

    // Assert
    match result {
        Err(ValidationError::InvalidContentPart { index: 0, .. }) => {
            // Expected error
        }
        other => panic!("Expected InvalidContentPart at index 0, got: {other:?}"),
    }
}

// ============================================================================
// Scenario: User message with attachment
// ============================================================================

/// When a user submits a message with an attachment,
/// the system should validate the attachment structure.
#[test]
fn user_message_with_attachment_is_accepted() {
    // Arrange
    let clock = DefaultClock;
    let validator = DefaultMessageValidator::new();

    let message = Message::new(
        ConversationId::new(),
        Role::User,
        vec![
            ContentPart::Text(TextPart::new("Please review this image.")),
            ContentPart::Attachment(
                AttachmentPart::new("image/png", "iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mNk+M9QDwADhgGAWjR9awAAAABJRU5ErkJggg==")
                    .with_name("screenshot.png")
                    .with_size(67),
            ),
        ],
        SequenceNumber::new(1),
        &clock,
    )
    .expect("message creation should succeed");

    // Act
    let result = validator.validate(&message);

    // Assert
    assert!(
        result.is_ok(),
        "Message with valid attachment should pass validation"
    );
}

// ============================================================================
// Scenario: Complete conversation flow
// ============================================================================

/// A complete conversation flow with user, assistant, and tool messages
/// should all validate successfully.
#[test]
fn complete_conversation_flow_validates() {
    // Arrange
    let clock = DefaultClock;
    let validator = DefaultMessageValidator::new();
    let conversation_id = ConversationId::new();

    // User asks a question
    let user_message = Message::new(
        conversation_id,
        Role::User,
        vec![ContentPart::Text(TextPart::new(
            "What's in the main.rs file?",
        ))],
        SequenceNumber::new(1),
        &clock,
    )
    .expect("user message");

    // Assistant decides to use a tool
    let assistant_message = Message::new(
        conversation_id,
        Role::Assistant,
        vec![
            ContentPart::Text(TextPart::new("I'll check that file for you.")),
            ContentPart::ToolCall(ToolCallPart::new(
                "call-001",
                "read_file",
                json!({"path": "src/main.rs"}),
            )),
        ],
        SequenceNumber::new(2),
        &clock,
    )
    .expect("assistant message");

    // Tool returns the result
    let tool_message = Message::new(
        conversation_id,
        Role::Tool,
        vec![ContentPart::ToolResult(ToolResultPart::success(
            "call-001",
            json!({"content": "fn main() { }"}),
        ))],
        SequenceNumber::new(3),
        &clock,
    )
    .expect("tool message");

    // Assistant summarises
    let final_message = Message::new(
        conversation_id,
        Role::Assistant,
        vec![ContentPart::Text(TextPart::new(
            "The main.rs file contains a simple main function.",
        ))],
        SequenceNumber::new(4),
        &clock,
    )
    .expect("final message");

    // Act & Assert
    assert!(
        validator.validate(&user_message).is_ok(),
        "User message should validate"
    );
    assert!(
        validator.validate(&assistant_message).is_ok(),
        "Assistant message should validate"
    );
    assert!(
        validator.validate(&tool_message).is_ok(),
        "Tool message should validate"
    );
    assert!(
        validator.validate(&final_message).is_ok(),
        "Final message should validate"
    );
}
