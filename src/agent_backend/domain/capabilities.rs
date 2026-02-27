//! Agent backend capability metadata.

use serde::{Deserialize, Serialize};

/// Describes the capabilities of a registered agent backend.
///
/// Capabilities are stored as JSONB so new fields can be added without
/// database migrations.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AgentCapabilities {
    supports_streaming: bool,
    supports_tool_calls: bool,
    supported_content_types: Vec<String>,
    max_context_window: Option<u64>,
}

impl AgentCapabilities {
    /// Creates capabilities with the two required boolean flags.
    ///
    /// `supported_content_types` defaults to an empty list and
    /// `max_context_window` defaults to `None`.
    #[must_use]
    pub const fn new(supports_streaming: bool, supports_tool_calls: bool) -> Self {
        Self {
            supports_streaming,
            supports_tool_calls,
            supported_content_types: Vec::new(),
            max_context_window: None,
        }
    }

    /// Sets the supported content types.
    #[must_use]
    pub fn with_content_types(mut self, types: impl IntoIterator<Item = String>) -> Self {
        self.supported_content_types = types.into_iter().collect();
        self
    }

    /// Sets the maximum context window size in tokens.
    #[must_use]
    pub const fn with_max_context_window(mut self, tokens: u64) -> Self {
        self.max_context_window = Some(tokens);
        self
    }

    /// Returns whether the backend supports streaming responses.
    #[must_use]
    pub const fn supports_streaming(&self) -> bool {
        self.supports_streaming
    }

    /// Returns whether the backend supports tool calls.
    #[must_use]
    pub const fn supports_tool_calls(&self) -> bool {
        self.supports_tool_calls
    }

    /// Returns the list of supported content types.
    #[must_use]
    pub fn supported_content_types(&self) -> &[String] {
        &self.supported_content_types
    }

    /// Returns the maximum context window size, if declared.
    #[must_use]
    pub const fn max_context_window(&self) -> Option<u64> {
        self.max_context_window
    }
}
