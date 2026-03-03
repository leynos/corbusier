//! Hook definitions and predicate configuration.

use super::{HookAction, HookDomainError, HookId, HookTriggerType};
use serde::{Deserialize, Serialize};

/// Priority ordering for hook execution.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(transparent)]
pub struct HookPriority(u16);

impl HookPriority {
    /// Creates a new priority value.
    ///
    /// Example: `HookPriority::new(10)` represents a higher priority.
    #[must_use]
    pub const fn new(value: u16) -> Self {
        Self(value)
    }

    /// Returns the priority as a numeric value.
    ///
    /// Example: `priority.value()` returns `10`.
    #[must_use]
    pub const fn value(self) -> u16 {
        self.0
    }
}

impl Default for HookPriority {
    fn default() -> Self {
        Self(100)
    }
}

/// Predicate configuration for matching hook triggers.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HookPredicate {
    data: serde_json::Value,
}

impl HookPredicate {
    /// Creates a predicate with the given structured data payload.
    ///
    /// Example: `HookPredicate::new(json!({\"tool\": \"rg\"}))` stores a
    /// predicate payload.
    #[must_use]
    pub const fn new(data: serde_json::Value) -> Self {
        Self { data }
    }

    /// Returns the predicate data payload.
    ///
    /// Example: `predicate.data()` returns the JSON payload.
    #[must_use]
    pub const fn data(&self) -> &serde_json::Value {
        &self.data
    }
}

impl Default for HookPredicate {
    fn default() -> Self {
        Self {
            data: serde_json::Value::Null,
        }
    }
}

/// Declarative hook definition.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HookDefinition {
    id: HookId,
    name: String,
    description: String,
    trigger: HookTriggerType,
    predicate: HookPredicate,
    actions: Vec<HookAction>,
    priority: HookPriority,
    enabled: bool,
}

impl HookDefinition {
    /// Creates a hook definition with required fields.
    ///
    /// Example: `HookDefinition::new(id, \"Hook\", trigger, actions)` builds a
    /// validated definition.
    ///
    /// # Errors
    ///
    /// Returns [`HookDomainError`] when the name is empty or no actions are
    /// provided.
    pub fn new(
        id: HookId,
        name: impl Into<String>,
        trigger: HookTriggerType,
        actions: Vec<HookAction>,
    ) -> Result<Self, HookDomainError> {
        let raw_name = name.into();
        let trimmed = raw_name.trim();
        if trimmed.is_empty() {
            return Err(HookDomainError::EmptyHookName);
        }
        if actions.is_empty() {
            return Err(HookDomainError::MissingActions);
        }
        Ok(Self {
            id,
            name: trimmed.to_owned(),
            description: String::new(),
            trigger,
            predicate: HookPredicate::default(),
            actions,
            priority: HookPriority::default(),
            enabled: true,
        })
    }

    /// Sets the description for the hook.
    ///
    /// Example: `definition.with_description(\"Checks\")` stores a description.
    #[must_use]
    pub fn with_description(mut self, description: impl Into<String>) -> Self {
        self.description = description.into();
        self
    }

    /// Sets the predicate for the hook.
    ///
    /// Example: `definition.with_predicate(predicate)` updates the predicate.
    #[must_use]
    pub fn with_predicate(mut self, predicate: HookPredicate) -> Self {
        self.predicate = predicate;
        self
    }

    /// Sets the priority ordering for the hook.
    ///
    /// Example: `definition.with_priority(HookPriority::new(1))` raises
    /// priority.
    #[must_use]
    pub const fn with_priority(mut self, priority: HookPriority) -> Self {
        self.priority = priority;
        self
    }

    /// Enables or disables the hook.
    ///
    /// Example: `definition.with_enabled(false)` disables the hook.
    #[must_use]
    pub const fn with_enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }

    /// Returns the hook identifier.
    ///
    /// Example: `definition.id()` returns the configured hook ID.
    #[must_use]
    pub const fn id(&self) -> &HookId {
        &self.id
    }

    /// Returns the hook name.
    ///
    /// Example: `definition.name()` returns the hook name.
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Returns the hook description.
    ///
    /// Example: `definition.description()` returns the hook description.
    #[must_use]
    pub fn description(&self) -> &str {
        &self.description
    }

    /// Returns the hook trigger type.
    ///
    /// Example: `definition.trigger()` returns `HookTriggerType::PreCommit`.
    #[must_use]
    pub const fn trigger(&self) -> HookTriggerType {
        self.trigger
    }

    /// Returns the predicate configuration.
    ///
    /// Example: `definition.predicate()` returns the predicate payload.
    #[must_use]
    pub const fn predicate(&self) -> &HookPredicate {
        &self.predicate
    }

    /// Returns the actions associated with the hook.
    ///
    /// Example: `definition.actions()` returns the configured actions.
    #[must_use]
    pub fn actions(&self) -> &[HookAction] {
        &self.actions
    }

    /// Returns the hook priority.
    ///
    /// Example: `definition.priority()` returns the priority value.
    #[must_use]
    pub const fn priority(&self) -> HookPriority {
        self.priority
    }

    /// Returns whether the hook is enabled.
    ///
    /// Example: `definition.is_enabled()` returns `true` for enabled hooks.
    #[must_use]
    pub const fn is_enabled(&self) -> bool {
        self.enabled
    }
}
