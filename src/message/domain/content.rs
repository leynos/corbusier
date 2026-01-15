//! Content part types representing the polymorphic content structure of messages.
//!
//! Messages contain a "parts" array that can include text, tool calls, and attachments.
//! This module defines the typed representation of these content variants.

use serde::{Deserialize, Serialize};
use serde_json::Value;

/// A single content part within a message.
///
/// Messages are composed of one or more content parts, allowing rich content
/// that combines text, tool interactions, and attachments.
///
/// # Serialisation
///
/// Content parts are serialised with a `type` tag field:
///
/// ```json
/// { "type": "text", "text": "Hello, world!" }
/// { "type": "tool_call", "call_id": "...", "name": "...", "arguments": {...} }
/// ```
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ContentPart {
    /// Plain text content.
    Text(TextPart),
    /// A tool call request from an assistant.
    ToolCall(ToolCallPart),
    /// A tool execution result.
    ToolResult(ToolResultPart),
    /// An attachment (file, image, etc.).
    Attachment(AttachmentPart),
}

/// Text content within a message.
///
/// # Examples
///
/// ```
/// use corbusier::message::domain::TextPart;
///
/// let text = TextPart::new("Hello, Corbusier!");
/// assert!(!text.is_empty());
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TextPart {
    /// The text content.
    pub text: String,
}

impl TextPart {
    /// Creates a new text part.
    #[must_use]
    pub fn new(text: impl Into<String>) -> Self {
        Self { text: text.into() }
    }

    /// Returns `true` if the text content is empty or whitespace-only.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.text.trim().is_empty()
    }

    /// Returns the length of the text content in bytes.
    #[must_use]
    pub const fn len(&self) -> usize {
        self.text.len()
    }
}

/// A tool call request within an assistant message.
///
/// Tool calls represent requests from the assistant to invoke external tools.
/// Each call has a unique identifier for matching with results.
///
/// # Examples
///
/// ```
/// use corbusier::message::domain::ToolCallPart;
/// use serde_json::json;
///
/// let call = ToolCallPart::new("call-123", "read_file", json!({"path": "/tmp/test.txt"}));
/// assert!(call.is_valid());
/// ```
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ToolCallPart {
    /// Unique identifier for this tool call (for matching with results).
    pub call_id: String,
    /// The name of the tool being invoked.
    pub name: String,
    /// Arguments passed to the tool as JSON.
    pub arguments: Value,
}

impl ToolCallPart {
    /// Creates a new tool call part.
    #[must_use]
    pub fn new(call_id: impl Into<String>, name: impl Into<String>, arguments: Value) -> Self {
        Self {
            call_id: call_id.into(),
            name: name.into(),
            arguments,
        }
    }

    /// Returns `true` if the tool call has valid structure.
    ///
    /// A valid tool call must have non-empty `call_id` and `name` fields.
    #[must_use]
    #[expect(
        clippy::missing_const_for_fn,
        reason = "String::is_empty is not const-stable"
    )]
    pub fn is_valid(&self) -> bool {
        !self.call_id.is_empty() && !self.name.is_empty()
    }
}

/// A tool execution result within a tool message.
///
/// Tool results carry the output of a tool invocation back to the assistant,
/// matched by `call_id` to the originating request.
///
/// # Examples
///
/// ```
/// use corbusier::message::domain::ToolResultPart;
/// use serde_json::json;
///
/// let result = ToolResultPart::success("call-123", json!({"content": "file data"}));
/// assert!(result.success);
/// ```
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ToolResultPart {
    /// The `call_id` this result corresponds to.
    pub call_id: String,
    /// The result content (can be structured JSON or plain text).
    pub content: Value,
    /// Whether the tool execution was successful.
    #[serde(default = "default_success")]
    pub success: bool,
}

const fn default_success() -> bool {
    true
}

impl ToolResultPart {
    /// Creates a successful tool result.
    #[must_use]
    pub fn success(call_id: impl Into<String>, content: Value) -> Self {
        Self {
            call_id: call_id.into(),
            content,
            success: true,
        }
    }

    /// Creates a failed tool result.
    #[must_use]
    pub fn failure(call_id: impl Into<String>, error: impl Into<String>) -> Self {
        Self {
            call_id: call_id.into(),
            content: Value::String(error.into()),
            success: false,
        }
    }

    /// Returns `true` if the tool call has a valid `call_id`.
    #[must_use]
    #[expect(
        clippy::missing_const_for_fn,
        reason = "String::is_empty is not const-stable"
    )]
    pub fn is_valid(&self) -> bool {
        !self.call_id.is_empty()
    }
}

/// An attachment within a message.
///
/// Attachments represent files, images, or other binary content embedded
/// in a message.
///
/// # Examples
///
/// ```
/// use corbusier::message::domain::AttachmentPart;
///
/// let attachment = AttachmentPart::new("text/plain", "SGVsbG8gV29ybGQ=")
///     .with_name("hello.txt")
///     .with_size(11);
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AttachmentPart {
    /// The MIME type of the attachment.
    pub mime_type: String,
    /// A display name for the attachment.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    /// The content (base64 encoded for binary data, or plain text).
    pub data: String,
    /// Size in bytes (for validation and display).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub size_bytes: Option<u64>,
}

impl AttachmentPart {
    /// Creates a new attachment part.
    #[must_use]
    pub fn new(mime_type: impl Into<String>, data: impl Into<String>) -> Self {
        Self {
            mime_type: mime_type.into(),
            name: None,
            data: data.into(),
            size_bytes: None,
        }
    }

    /// Sets the display name for the attachment.
    #[must_use]
    pub fn with_name(mut self, name: impl Into<String>) -> Self {
        self.name = Some(name.into());
        self
    }

    /// Sets the size in bytes.
    #[must_use]
    pub const fn with_size(mut self, size_bytes: u64) -> Self {
        self.size_bytes = Some(size_bytes);
        self
    }

    /// Returns `true` if the attachment has valid structure.
    ///
    /// A valid attachment must have non-empty `mime_type` and `data` fields.
    #[must_use]
    #[expect(
        clippy::missing_const_for_fn,
        reason = "String::is_empty is not const-stable"
    )]
    pub fn is_valid(&self) -> bool {
        !self.mime_type.is_empty() && !self.data.is_empty()
    }
}
