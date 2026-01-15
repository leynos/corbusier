//! Domain error types for message validation and processing.
//!
//! Uses `thiserror` for ergonomic error handling with typed variants
//! that can be inspected by callers.

use super::domain::{MessageId, SequenceNumber};
use std::sync::Arc;
use thiserror::Error;

/// Errors that can occur during message validation.
#[derive(Debug, Clone, Error)]
pub enum ValidationError {
    /// The message ID is missing or invalid.
    #[error("message ID is required")]
    MissingMessageId,

    /// The role is invalid for this context.
    #[error("invalid role '{0}' for this message type")]
    InvalidRole(String),

    /// The content parts array is empty.
    #[error("message must contain at least one content part")]
    EmptyContent,

    /// A content part has invalid structure.
    #[error("invalid content part at index {index}: {reason}")]
    InvalidContentPart {
        /// The index of the invalid part.
        index: usize,
        /// Description of the validation failure.
        reason: String,
    },

    /// The timestamp is missing.
    #[error("message timestamp is required")]
    MissingTimestamp,

    /// A text content part is empty or whitespace-only.
    #[error("text content cannot be empty")]
    EmptyTextContent,

    /// A tool call has invalid structure.
    #[error("invalid tool call: {0}")]
    InvalidToolCall(String),

    /// An attachment has invalid structure.
    #[error("invalid attachment: {0}")]
    InvalidAttachment(String),

    /// Metadata validation failed.
    #[error("invalid metadata: {0}")]
    InvalidMetadata(String),

    /// The message sequence is out of order.
    #[error("message sequence {actual} is invalid; expected {expected}")]
    InvalidSequence {
        /// The actual sequence number.
        actual: SequenceNumber,
        /// The expected sequence number.
        expected: SequenceNumber,
    },

    /// A duplicate message was detected.
    #[error("duplicate message ID: {0}")]
    DuplicateMessage(MessageId),

    /// The message exceeds size limits.
    #[error("message size {actual_bytes} exceeds limit of {limit_bytes} bytes")]
    MessageTooLarge {
        /// The actual size in bytes.
        actual_bytes: usize,
        /// The maximum allowed size.
        limit_bytes: usize,
    },

    /// The message has too many content parts.
    #[error("message has {actual} content parts, exceeds limit of {max}")]
    TooManyContentParts {
        /// The maximum allowed number of content parts.
        max: usize,
        /// The actual number of content parts.
        actual: usize,
    },

    /// The message references a non-existent conversation.
    #[error("conversation not found")]
    ConversationNotFound,

    /// Multiple validation errors occurred.
    #[error("multiple validation errors: {}", format_errors(.0))]
    Multiple(Vec<Self>),
}

fn format_errors(errors: &[ValidationError]) -> String {
    errors
        .iter()
        .map(ToString::to_string)
        .collect::<Vec<_>>()
        .join("; ")
}

impl ValidationError {
    /// Creates a validation error for an invalid content part.
    #[must_use]
    pub fn invalid_content_part(index: usize, reason: impl Into<String>) -> Self {
        Self::InvalidContentPart {
            index,
            reason: reason.into(),
        }
    }

    /// Combines multiple validation errors into a single error.
    ///
    /// If only one error is provided, returns it directly rather than wrapping.
    ///
    /// # Panics
    ///
    /// Panics in debug builds if called with an empty vector, as this indicates
    /// a logic error in the caller. In release builds, returns an internal
    /// error variant.
    #[must_use]
    pub fn multiple(errors: Vec<Self>) -> Self {
        match errors.len() {
            0 => {
                debug_assert!(false, "multiple() called with empty errors vector");
                Self::InvalidMetadata("internal error: no validation errors".into())
            }
            1 => {
                // Length is verified to be 1 immediately above, so this will always succeed.
                errors.into_iter().next().unwrap_or_else(|| {
                    Self::InvalidMetadata("internal error: no validation errors".into())
                })
            }
            _ => Self::Multiple(errors),
        }
    }

    /// Returns `true` if this error represents multiple validation failures.
    #[must_use]
    pub const fn is_multiple(&self) -> bool {
        matches!(self, Self::Multiple(_))
    }

    /// Returns the individual errors if this is a `Multiple` variant.
    #[must_use]
    pub fn errors(&self) -> Option<&[Self]> {
        match self {
            Self::Multiple(errors) => Some(errors),
            _ => None,
        }
    }
}

/// Errors that can occur during message persistence.
#[derive(Debug, Error)]
pub enum RepositoryError {
    /// The message was not found.
    #[error("message not found: {0}")]
    NotFound(MessageId),

    /// A message with this ID already exists.
    #[error("duplicate message: {0}")]
    DuplicateMessage(MessageId),

    /// A message with this sequence number already exists in the conversation.
    #[error("duplicate sequence number {sequence} in conversation {conversation_id}")]
    DuplicateSequence {
        /// The conversation containing the conflict.
        conversation_id: super::domain::ConversationId,
        /// The conflicting sequence number.
        sequence: SequenceNumber,
    },

    /// A database error occurred.
    #[error("database error: {0}")]
    Database(Arc<dyn std::error::Error + Send + Sync>),

    /// A serialization error occurred.
    #[error("serialization error: {0}")]
    Serialization(String),

    /// A connection error occurred.
    #[error("connection error: {0}")]
    Connection(String),
}

impl RepositoryError {
    /// Creates a database error from any error type.
    #[must_use]
    pub fn database(err: impl std::error::Error + Send + Sync + 'static) -> Self {
        Self::Database(Arc::new(err))
    }

    /// Creates a serialization error.
    #[must_use]
    pub fn serialization(message: impl Into<String>) -> Self {
        Self::Serialization(message.into())
    }

    /// Creates a connection error.
    #[must_use]
    pub fn connection(message: impl Into<String>) -> Self {
        Self::Connection(message.into())
    }
}

impl From<diesel::result::Error> for RepositoryError {
    fn from(err: diesel::result::Error) -> Self {
        // All Diesel errors are converted to database errors.
        // Unique constraint violations are identified but cannot provide
        // semantic errors (DuplicateMessage/DuplicateSequence) since the
        // constraint error doesn't include the specific IDs. Callers should
        // use pre-check validation to get semantic errors with correct identifiers.
        Self::database(err)
    }
}

/// Errors that can occur during schema version upgrades.
#[derive(Debug, Error)]
pub enum SchemaUpgradeError {
    /// The schema version is not supported.
    #[error("unsupported schema version: {0}")]
    UnsupportedVersion(u32),

    /// The event type is not recognized.
    #[error("unknown event type: {0}")]
    UnknownEventType(String),

    /// The upgrade failed.
    #[error("upgrade from version {from} to {to} failed: {reason}")]
    UpgradeFailed {
        /// The source version.
        from: u32,
        /// The target version.
        to: u32,
        /// Description of the failure.
        reason: String,
    },

    /// The event data is malformed.
    #[error("malformed event data: {0}")]
    MalformedData(String),
}

impl SchemaUpgradeError {
    /// Creates an upgrade failed error.
    #[must_use]
    pub fn upgrade_failed(from: u32, to: u32, reason: impl Into<String>) -> Self {
        Self::UpgradeFailed {
            from,
            to,
            reason: reason.into(),
        }
    }

    /// Creates a malformed data error.
    #[must_use]
    pub fn malformed(message: impl Into<String>) -> Self {
        Self::MalformedData(message.into())
    }
}
