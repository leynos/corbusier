//! Validator port for message validation.
//!
//! Defines the abstract interface for validating messages at different layers.

use crate::message::{domain::Message, error::ValidationError};

/// Result type for validation operations.
pub type ValidationResult<T> = Result<T, ValidationError>;

/// Port for message validation operations.
///
/// Validation occurs in layers:
/// 1. Structure validation (required fields, types)
/// 2. Content validation (text, tool calls, attachments)
/// 3. Business rule validation (sequence, duplicates, context)
///
/// # Implementation Notes
///
/// Implementations should:
/// - Collect all validation errors before returning (not fail-fast)
/// - Use `ValidationError::multiple` to combine errors
/// - Be stateless and thread-safe
pub trait MessageValidator: Send + Sync {
    /// Validates a message against all rules.
    ///
    /// # Errors
    ///
    /// Returns `ValidationError` if any validation rule fails.
    /// Multiple failures are combined using `ValidationError::Multiple`.
    fn validate(&self, message: &Message) -> ValidationResult<()>;

    /// Validates only the structural aspects of a message.
    ///
    /// Checks:
    /// - Message ID is non-nil
    /// - Content array is non-empty
    /// - Required fields are present
    ///
    /// # Errors
    ///
    /// Returns `ValidationError` if structural validation fails.
    fn validate_structure(&self, message: &Message) -> ValidationResult<()>;

    /// Validates the content parts of a message.
    ///
    /// Checks:
    /// - Text parts are non-empty (if configured)
    /// - Tool calls have valid structure
    /// - Attachments have valid MIME types and data
    ///
    /// # Errors
    ///
    /// Returns `ValidationError` if content validation fails.
    fn validate_content(&self, message: &Message) -> ValidationResult<()>;
}

/// Configuration for validation rules.
///
/// Allows customization of validation behaviour for different contexts.
///
/// # Examples
///
/// ```
/// use corbusier::message::ports::validator::ValidationConfig;
///
/// let config = ValidationConfig::default();
/// assert!(!config.allow_empty_text);
///
/// let lenient = ValidationConfig::lenient();
/// assert!(lenient.allow_empty_text);
/// ```
#[derive(Debug, Clone)]
pub struct ValidationConfig {
    /// Maximum message size in bytes.
    pub max_message_size_bytes: usize,
    /// Maximum number of content parts.
    pub max_content_parts: usize,
    /// Maximum text content length in characters.
    pub max_text_length: usize,
    /// Whether to allow empty text parts.
    pub allow_empty_text: bool,
}

impl Default for ValidationConfig {
    fn default() -> Self {
        Self {
            max_message_size_bytes: 1024 * 1024, // 1 MiB
            max_content_parts: 100,
            max_text_length: 100_000,
            allow_empty_text: false,
        }
    }
}

impl ValidationConfig {
    /// Creates a lenient configuration that allows empty text.
    ///
    /// Useful for testing or when relaxed validation is acceptable.
    #[must_use]
    pub fn lenient() -> Self {
        Self {
            allow_empty_text: true,
            ..Default::default()
        }
    }

    /// Creates a strict configuration with reduced limits.
    ///
    /// Useful for resource-constrained environments.
    #[must_use]
    pub const fn strict() -> Self {
        Self {
            max_message_size_bytes: 256 * 1024, // 256 KiB
            max_content_parts: 20,
            max_text_length: 10_000,
            allow_empty_text: false,
        }
    }
}
