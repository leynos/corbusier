//! Hook action definitions.

use super::HookActionId;
use serde::{Deserialize, Serialize};

/// Declarative hook action configuration.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HookAction {
    id: HookActionId,
    action_type: HookActionType,
    configuration: serde_json::Value,
}

impl HookAction {
    /// Creates a hook action with default configuration.
    ///
    /// Example: `HookAction::new(id, HookActionType::QualityGate)` creates an
    /// action with empty configuration.
    #[must_use]
    pub const fn new(id: HookActionId, action_type: HookActionType) -> Self {
        Self {
            id,
            action_type,
            configuration: serde_json::Value::Null,
        }
    }

    /// Attaches configuration for the action.
    ///
    /// Example: `action.with_configuration(json!({\"key\": \"value\"}))` sets
    /// the configuration payload.
    #[must_use]
    pub fn with_configuration(mut self, configuration: serde_json::Value) -> Self {
        self.configuration = configuration;
        self
    }

    /// Returns the action identifier.
    ///
    /// Example: `action.id()` returns the configured action ID.
    #[must_use]
    pub const fn id(&self) -> &HookActionId {
        &self.id
    }

    /// Returns the action type.
    ///
    /// Example: `action.action_type()` returns `HookActionType::QualityGate`.
    #[must_use]
    pub const fn action_type(&self) -> &HookActionType {
        &self.action_type
    }

    /// Returns the action configuration payload.
    ///
    /// Example: `action.configuration()` returns the JSON payload.
    #[must_use]
    pub const fn configuration(&self) -> &serde_json::Value {
        &self.configuration
    }
}

/// Supported hook action kinds.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HookActionType {
    /// Executes a quality gate (lint, tests, etc.).
    QualityGate,
    /// Runs a policy check.
    PolicyCheck,
    /// Sends a notification.
    Notification,
    /// Blocks a workflow action.
    BlockAction,
    /// Runs remediation steps.
    Remediation,
}

impl HookActionType {
    /// Returns the stable string representation for persistence and logs.
    ///
    /// Example: `HookActionType::QualityGate.as_str()` returns `"quality_gate"`.
    #[must_use]
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::QualityGate => "quality_gate",
            Self::PolicyCheck => "policy_check",
            Self::Notification => "notification",
            Self::BlockAction => "block_action",
            Self::Remediation => "remediation",
        }
    }
}
