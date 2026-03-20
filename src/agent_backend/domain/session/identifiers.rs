//! Turn-session identifier value objects.

use super::TurnSessionDomainError;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Unique identifier for a turn orchestration session.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct TurnSessionId(Uuid);

impl TurnSessionId {
    /// Creates a new random turn-session identifier.
    #[must_use]
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }

    /// Creates an identifier from an existing UUID.
    #[must_use]
    pub const fn from_uuid(value: Uuid) -> Self {
        Self(value)
    }

    /// Returns the wrapped UUID.
    #[must_use]
    pub const fn into_inner(self) -> Uuid {
        self.0
    }
}

impl Default for TurnSessionId {
    fn default() -> Self {
        Self::new()
    }
}

/// Backend-native runtime session identifier.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct RuntimeSessionId(String);

impl RuntimeSessionId {
    /// Creates a validated runtime session identifier.
    ///
    /// # Errors
    ///
    /// Returns [`TurnSessionDomainError::EmptyRuntimeSessionId`] when the
    /// identifier is empty after trimming.
    pub fn new(value: impl Into<String>) -> Result<Self, TurnSessionDomainError> {
        let raw_value = value.into();
        if raw_value.trim().is_empty() {
            return Err(TurnSessionDomainError::EmptyRuntimeSessionId);
        }
        Ok(Self(raw_value))
    }

    /// Returns the runtime session identifier as a string slice.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Returns the wrapped identifier as an owned string.
    #[must_use]
    pub fn into_inner(self) -> String {
        self.0
    }
}

impl AsRef<str> for RuntimeSessionId {
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}

impl TryFrom<String> for RuntimeSessionId {
    type Error = TurnSessionDomainError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        Self::new(value)
    }
}

impl From<RuntimeSessionId> for String {
    fn from(value: RuntimeSessionId) -> Self {
        value.into_inner()
    }
}
