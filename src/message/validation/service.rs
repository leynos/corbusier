//! Validation service implementation.
//!
//! Provides the default implementation of the `MessageValidator` port,
//! combining individual validation rules into a comprehensive validator.

use crate::message::{
    domain::Message,
    error::ValidationError,
    ports::validator::{MessageValidator, ValidationConfig, ValidationResult},
    validation::rules,
};

/// Default implementation of the message validator.
///
/// Applies all validation rules in order, collecting errors to provide
/// comprehensive feedback rather than failing on the first error.
///
/// # Examples
///
/// ```
/// use corbusier::message::domain::{
///     ContentPart, ConversationId, Message, Role, SequenceNumber, TextPart,
/// };
/// use corbusier::message::ports::validator::MessageValidator;
/// use corbusier::message::validation::service::DefaultMessageValidator;
/// use mockable::DefaultClock;
///
/// let clock = DefaultClock;
/// let message = Message::new(
///     ConversationId::new(),
///     Role::User,
///     vec![ContentPart::Text(TextPart::new("Hello"))],
///     SequenceNumber::new(1),
///     &clock,
/// ).expect("valid message");
///
/// let validator = DefaultMessageValidator::new();
/// assert!(validator.validate(&message).is_ok());
/// ```
#[derive(Debug, Clone)]
pub struct DefaultMessageValidator {
    config: ValidationConfig,
}

impl DefaultMessageValidator {
    /// Creates a new validator with default configuration.
    #[must_use]
    pub fn new() -> Self {
        Self {
            config: ValidationConfig::default(),
        }
    }

    /// Creates a new validator with custom configuration.
    #[must_use]
    pub const fn with_config(config: ValidationConfig) -> Self {
        Self { config }
    }

    /// Returns the current validation configuration.
    #[must_use]
    pub const fn config(&self) -> &ValidationConfig {
        &self.config
    }
}

impl Default for DefaultMessageValidator {
    fn default() -> Self {
        Self::new()
    }
}

impl MessageValidator for DefaultMessageValidator {
    fn validate(&self, message: &Message) -> ValidationResult<()> {
        let mut errors = Vec::new();

        if let Err(e) = self.validate_structure(message) {
            collect_errors(&mut errors, e);
        }

        if let Err(e) = self.validate_content(message) {
            collect_errors(&mut errors, e);
        }

        if let Err(e) = rules::validate_message_size(message, &self.config) {
            errors.push(e);
        }

        if errors.is_empty() {
            Ok(())
        } else {
            Err(ValidationError::multiple(errors))
        }
    }

    fn validate_structure(&self, message: &Message) -> ValidationResult<()> {
        let mut errors = Vec::new();

        if let Err(e) = rules::validate_message_id(message) {
            errors.push(e);
        }

        if let Err(e) = rules::validate_content_not_empty(message) {
            errors.push(e);
        }

        if let Err(e) = rules::validate_content_parts_count(message, &self.config) {
            errors.push(e);
        }

        if errors.is_empty() {
            Ok(())
        } else {
            Err(ValidationError::multiple(errors))
        }
    }

    fn validate_content(&self, message: &Message) -> ValidationResult<()> {
        rules::validate_content_parts(message, &self.config)
    }
}

/// Helper function to collect errors, flattening `Multiple` variants.
fn collect_errors(errors: &mut Vec<ValidationError>, error: ValidationError) {
    match error {
        ValidationError::Multiple(inner) => errors.extend(inner),
        other => errors.push(other),
    }
}

// Note: Unit tests for DefaultMessageValidator are located in
// src/message/tests/validation_tests.rs with comprehensive coverage
// using rstest fixtures.
