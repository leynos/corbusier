//! Governance decision domain types for tool call authorization.
//!
//! A [`ToolGovernanceDecision`] represents the outcome of evaluating a tool
//! call against a pluggable governance enforcement point.

/// Outcome of a governance evaluation for a tool call.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ToolGovernanceDecision {
    /// The tool call is permitted.
    Allow,
    /// The tool call is denied with an explanation.
    Deny {
        /// Human-readable reason for denial.
        reason: String,
    },
}

impl ToolGovernanceDecision {
    /// Returns `true` when the decision permits the tool call.
    #[must_use]
    pub const fn is_allowed(&self) -> bool {
        matches!(self, Self::Allow)
    }

    /// Returns the denial reason, if the decision is `Deny`.
    #[must_use]
    pub fn denial_reason(&self) -> Option<&str> {
        match self {
            Self::Allow => None,
            Self::Deny { reason } => Some(reason),
        }
    }
}
