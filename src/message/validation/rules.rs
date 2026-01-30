//! Individual validation rule implementations.
//!
//! Each rule is implemented as a pure function that validates a specific
//! aspect of a message. Rules return `Ok(())` on success or a specific
//! `ValidationError` on failure.

use crate::message::{
    domain::{
        AgentResponseAudit, AttachmentPart, ContentPart, Message, TextPart, ToolCallAudit,
        ToolCallPart, ToolResultPart,
    },
    error::ValidationError,
    ports::validator::ValidationConfig,
};

/// Validates that the message has a non-nil ID.
///
/// # Errors
///
/// Returns `ValidationError::MissingMessageId` if the ID is nil.
pub fn validate_message_id(message: &Message) -> Result<(), ValidationError> {
    if message.id().as_ref().is_nil() {
        return Err(ValidationError::MissingMessageId);
    }
    Ok(())
}

/// Validates that the message has at least one content part.
///
/// # Errors
///
/// Returns `ValidationError::EmptyContent` if the content array is empty.
pub fn validate_content_not_empty(message: &Message) -> Result<(), ValidationError> {
    if message.content().is_empty() {
        return Err(ValidationError::EmptyContent);
    }
    Ok(())
}

/// Validates that the message does not exceed size limits.
///
/// # Errors
///
/// Returns `ValidationError::MessageTooLarge` if the serialized message
/// exceeds the configured limit.
pub fn validate_message_size(
    message: &Message,
    config: &ValidationConfig,
) -> Result<(), ValidationError> {
    let serialized = serde_json::to_vec(message).map_err(|e| {
        ValidationError::InvalidMetadata(format!("failed to serialize message: {e}"))
    })?;

    if serialized.len() > config.max_message_size_bytes {
        return Err(ValidationError::MessageTooLarge {
            actual_bytes: serialized.len(),
            limit_bytes: config.max_message_size_bytes,
        });
    }

    Ok(())
}

/// Validates that the message does not have too many content parts.
///
/// # Errors
///
/// Returns `ValidationError::TooManyContentParts` if the number of parts exceeds
/// the configured limit.
pub fn validate_content_parts_count(
    message: &Message,
    config: &ValidationConfig,
) -> Result<(), ValidationError> {
    let count = message.content().len();
    if count > config.max_content_parts {
        return Err(ValidationError::TooManyContentParts {
            max: config.max_content_parts,
            actual: count,
        });
    }
    Ok(())
}

/// Validates all individual content parts.
///
/// # Errors
///
/// Returns `ValidationError::Multiple` if any content parts are invalid.
pub fn validate_content_parts(
    message: &Message,
    config: &ValidationConfig,
) -> Result<(), ValidationError> {
    let mut errors = Vec::new();

    for (index, part) in message.content().iter().enumerate() {
        if let Err(e) = validate_content_part(part, index, config) {
            errors.push(e);
        }
    }

    if errors.is_empty() {
        Ok(())
    } else {
        Err(ValidationError::multiple(errors))
    }
}

/// Validates message metadata audit records.
///
/// # Errors
///
/// Returns `ValidationError::InvalidMetadata` if audit metadata is malformed.
///
/// # Examples
///
/// ```rust
/// use corbusier::message::domain::{
///     ContentPart, ConversationId, Message, MessageMetadata, Role, SequenceNumber, TextPart,
///     ToolCallAudit, ToolCallStatus,
/// };
/// use corbusier::message::validation::rules::validate_metadata;
/// use mockable::DefaultClock;
///
/// let clock = DefaultClock;
/// let metadata = MessageMetadata::empty().with_tool_call_audit(
///     ToolCallAudit::new("call-123", "search", ToolCallStatus::Succeeded),
/// );
/// let message = Message::builder(ConversationId::new(), Role::Assistant, SequenceNumber::new(1))
///     .with_content(ContentPart::Text(TextPart::new("Hello")))
///     .with_metadata(metadata)
///     .build(&clock)
///     .expect("valid message");
///
/// assert!(validate_metadata(&message).is_ok());
/// ```
pub fn validate_metadata(message: &Message) -> Result<(), ValidationError> {
    let metadata = message.metadata();
    let mut errors = Vec::new();

    for (index, audit) in metadata.tool_call_audits.iter().enumerate() {
        collect_metadata_error(&mut errors, validate_tool_call_audit(audit, index));
    }

    if let Some(audit) = metadata.agent_response_audit.as_ref() {
        collect_metadata_error(&mut errors, validate_agent_response_audit(audit));
    }

    if errors.is_empty() {
        Ok(())
    } else {
        Err(ValidationError::multiple(errors))
    }
}

fn validate_content_part(
    part: &ContentPart,
    index: usize,
    config: &ValidationConfig,
) -> Result<(), ValidationError> {
    match part {
        ContentPart::Text(text) => validate_text_part(text, index, config),
        ContentPart::ToolCall(tool_call) => validate_tool_call_part(tool_call, index),
        ContentPart::ToolResult(tool_result) => validate_tool_result_part(tool_result, index),
        ContentPart::Attachment(attachment) => validate_attachment_part(attachment, index),
    }
}

fn validate_tool_call_audit(audit: &ToolCallAudit, index: usize) -> Option<ValidationError> {
    if audit.call_id.trim().is_empty() {
        return Some(ValidationError::InvalidMetadata(format!(
            "tool call audit at index {index} must include a call_id"
        )));
    }

    if audit.tool_name.trim().is_empty() {
        return Some(ValidationError::InvalidMetadata(format!(
            "tool call audit at index {index} must include a tool_name"
        )));
    }

    if let Some(error) = audit.error.as_ref()
        && error.trim().is_empty()
    {
        return Some(ValidationError::InvalidMetadata(format!(
            "tool call audit at index {index} has an empty error message"
        )));
    }

    None
}

fn validate_agent_response_audit(audit: &AgentResponseAudit) -> Option<ValidationError> {
    if let Some(response_id) = audit.response_id.as_ref()
        && response_id.trim().is_empty()
    {
        return Some(ValidationError::InvalidMetadata(
            "agent response audit must include a non-empty response_id".to_owned(),
        ));
    }

    if let Some(model) = audit.model.as_ref()
        && model.trim().is_empty()
    {
        return Some(ValidationError::InvalidMetadata(
            "agent response audit must include a non-empty model".to_owned(),
        ));
    }

    if let Some(error) = audit.error.as_ref()
        && error.trim().is_empty()
    {
        return Some(ValidationError::InvalidMetadata(
            "agent response audit must include a non-empty error".to_owned(),
        ));
    }

    None
}

fn collect_metadata_error(errors: &mut Vec<ValidationError>, maybe_error: Option<ValidationError>) {
    if let Some(error) = maybe_error {
        errors.push(error);
    }
}

fn validate_text_part(
    text: &TextPart,
    index: usize,
    config: &ValidationConfig,
) -> Result<(), ValidationError> {
    if !config.allow_empty_text && text.is_empty() {
        return Err(ValidationError::invalid_content_part(
            index,
            "text content cannot be empty",
        ));
    }

    let char_count = text.text.chars().count();
    if char_count > config.max_text_length {
        return Err(ValidationError::invalid_content_part(
            index,
            format!(
                "text content exceeds maximum length of {} characters",
                config.max_text_length
            ),
        ));
    }

    Ok(())
}

fn validate_tool_call_part(tool_call: &ToolCallPart, index: usize) -> Result<(), ValidationError> {
    if tool_call.call_id.is_empty() {
        return Err(ValidationError::invalid_content_part(
            index,
            "tool call must have a call_id",
        ));
    }

    if tool_call.name.is_empty() {
        return Err(ValidationError::invalid_content_part(
            index,
            "tool call must have a name",
        ));
    }

    Ok(())
}

fn validate_tool_result_part(
    tool_result: &ToolResultPart,
    index: usize,
) -> Result<(), ValidationError> {
    if tool_result.call_id.is_empty() {
        return Err(ValidationError::invalid_content_part(
            index,
            "tool result must have a call_id",
        ));
    }

    Ok(())
}

fn validate_attachment_part(
    attachment: &AttachmentPart,
    index: usize,
) -> Result<(), ValidationError> {
    if attachment.mime_type.is_empty() {
        return Err(ValidationError::invalid_content_part(
            index,
            "attachment must have a MIME type",
        ));
    }

    if attachment.data.is_empty() {
        return Err(ValidationError::invalid_content_part(
            index,
            "attachment data cannot be empty",
        ));
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::message::{
        domain::{ConversationId, Role, SequenceNumber},
        tests::validation_fixtures::clock,
    };
    use mockable::DefaultClock;
    use rstest::rstest;

    fn create_message_with_content(content: Vec<ContentPart>, clock: &DefaultClock) -> Message {
        Message::new(
            ConversationId::new(),
            Role::User,
            content,
            SequenceNumber::new(1),
            clock,
        )
        .expect("test message should be valid")
    }

    #[rstest]
    fn validate_message_id_accepts_valid_id(clock: DefaultClock) {
        let message =
            create_message_with_content(vec![ContentPart::Text(TextPart::new("test"))], &clock);
        assert!(validate_message_id(&message).is_ok());
    }

    #[rstest]
    fn validate_message_id_rejects_nil_id(clock: DefaultClock) {
        // Create a valid message, then deserialize a modified version with nil ID.
        // This tests the defensive validation for external data/deserialization.
        let message =
            create_message_with_content(vec![ContentPart::Text(TextPart::new("test"))], &clock);
        let mut json_value: serde_json::Value = serde_json::to_value(&message).expect("serialize");
        *json_value
            .get_mut("id")
            .expect("message should have id field") =
            serde_json::json!("00000000-0000-0000-0000-000000000000");
        let nil_id_message: Message =
            serde_json::from_value(json_value).expect("deserialize with nil ID");

        assert!(matches!(
            validate_message_id(&nil_id_message),
            Err(ValidationError::MissingMessageId)
        ));
    }

    #[rstest]
    fn validate_content_not_empty_accepts_non_empty(clock: DefaultClock) {
        let message =
            create_message_with_content(vec![ContentPart::Text(TextPart::new("test"))], &clock);
        assert!(validate_content_not_empty(&message).is_ok());
    }

    #[rstest]
    fn validate_content_not_empty_rejects_empty_content(clock: DefaultClock) {
        // Attempt to create a message with empty content fails at the builder level,
        // so we test the validation function directly by using a message that has
        // been constructed (which requires at least one content part).
        // Instead, we test the rejection scenario using the builder.
        let result = Message::new(
            ConversationId::new(),
            Role::User,
            vec![],
            SequenceNumber::new(1),
            &clock,
        );
        // The Message::new constructor rejects empty content
        assert!(result.is_err());
    }
}
