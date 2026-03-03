//! Identifier types for hook engine domain entities.

use super::HookDomainError;
use serde::{Deserialize, Serialize};
use std::fmt;
use uuid::Uuid;

/// Unique identifier for a hook definition.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, PartialOrd, Ord)]
#[serde(try_from = "String", into = "String")]
pub struct HookId(String);

impl HookId {
    /// Creates a new validated hook identifier.
    ///
    /// Example: `HookId::new("hook-1")` returns `Ok(_)`.
    ///
    /// # Errors
    ///
    /// Returns [`HookDomainError::EmptyHookId`] when the identifier is empty.
    pub fn new(value: impl Into<String>) -> Result<Self, HookDomainError> {
        let raw = value.into();
        let trimmed = raw.trim();
        if trimmed.is_empty() {
            return Err(HookDomainError::EmptyHookId);
        }
        Ok(Self(trimmed.to_owned()))
    }

    /// Returns the identifier as a string slice.
    ///
    /// Example: `hook_id.as_str()` returns `"hook-1"`.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Returns the underlying identifier string.
    ///
    /// Example: `hook_id.into_inner()` returns the owned identifier.
    #[must_use]
    pub fn into_inner(self) -> String {
        self.0
    }
}

impl AsRef<str> for HookId {
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}

impl TryFrom<&str> for HookId {
    type Error = HookDomainError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        Self::new(value)
    }
}

impl TryFrom<String> for HookId {
    type Error = HookDomainError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        Self::new(value)
    }
}

impl From<HookId> for String {
    fn from(value: HookId) -> Self {
        value.into_inner()
    }
}

impl fmt::Display for HookId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Unique identifier for an action within a hook definition.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, PartialOrd, Ord)]
#[serde(try_from = "String", into = "String")]
pub struct HookActionId(String);

impl HookActionId {
    /// Creates a new validated hook action identifier.
    ///
    /// Example: `HookActionId::new("action-1")` returns `Ok(_)`.
    ///
    /// # Errors
    ///
    /// Returns [`HookDomainError::EmptyHookActionId`] when the identifier is empty.
    pub fn new(value: impl Into<String>) -> Result<Self, HookDomainError> {
        let raw = value.into();
        let trimmed = raw.trim();
        if trimmed.is_empty() {
            return Err(HookDomainError::EmptyHookActionId);
        }
        Ok(Self(trimmed.to_owned()))
    }

    /// Returns the identifier as a string slice.
    ///
    /// Example: `action_id.as_str()` returns `"action-1"`.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Returns the underlying identifier string.
    ///
    /// Example: `action_id.into_inner()` returns the owned identifier.
    #[must_use]
    pub fn into_inner(self) -> String {
        self.0
    }
}

impl AsRef<str> for HookActionId {
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}

impl TryFrom<&str> for HookActionId {
    type Error = HookDomainError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        Self::new(value)
    }
}

impl TryFrom<String> for HookActionId {
    type Error = HookDomainError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        Self::new(value)
    }
}

impl From<HookActionId> for String {
    fn from(value: HookActionId) -> Self {
        value.into_inner()
    }
}

impl fmt::Display for HookActionId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Unique identifier for a hook execution result.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct HookExecutionId(Uuid);

impl HookExecutionId {
    /// Creates a new random execution identifier.
    ///
    /// Example: `HookExecutionId::new()` returns a fresh identifier.
    #[must_use]
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }

    /// Creates an execution identifier from an existing UUID.
    ///
    /// Example: `HookExecutionId::from_uuid(uuid)` wraps the UUID.
    #[must_use]
    pub const fn from_uuid(uuid: Uuid) -> Self {
        Self(uuid)
    }

    /// Returns the wrapped UUID.
    ///
    /// Example: `execution_id.into_inner()` returns the UUID.
    #[must_use]
    pub const fn into_inner(self) -> Uuid {
        self.0
    }
}

impl Default for HookExecutionId {
    fn default() -> Self {
        Self::new()
    }
}

impl AsRef<Uuid> for HookExecutionId {
    fn as_ref(&self) -> &Uuid {
        &self.0
    }
}

impl fmt::Display for HookExecutionId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Correlation identifier for a trigger invocation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct TriggerContextId(Uuid);

impl TriggerContextId {
    /// Creates a new random trigger context identifier.
    ///
    /// Example: `TriggerContextId::new()` returns a fresh identifier.
    #[must_use]
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }

    /// Creates a trigger context identifier from an existing UUID.
    ///
    /// Example: `TriggerContextId::from_uuid(uuid)` wraps the UUID.
    #[must_use]
    pub const fn from_uuid(uuid: Uuid) -> Self {
        Self(uuid)
    }

    /// Returns the wrapped UUID.
    ///
    /// Example: `context_id.into_inner()` returns the UUID.
    #[must_use]
    pub const fn into_inner(self) -> Uuid {
        self.0
    }
}

impl Default for TriggerContextId {
    fn default() -> Self {
        Self::new()
    }
}

impl AsRef<Uuid> for TriggerContextId {
    fn as_ref(&self) -> &Uuid {
        &self.0
    }
}

impl fmt::Display for TriggerContextId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}
