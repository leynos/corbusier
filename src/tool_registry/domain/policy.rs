//! Policy decision domain types for tool call authorisation.
//!
//! A [`PolicyDecision`] represents the outcome of evaluating a tool call
//! against a pluggable policy enforcement point.

/// Outcome of a policy evaluation for a tool call.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PolicyDecision {
    /// The tool call is permitted.
    Allow,
    /// The tool call is denied with an explanation.
    Deny {
        /// Human-readable reason for denial.
        reason: String,
    },
}

impl PolicyDecision {
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
