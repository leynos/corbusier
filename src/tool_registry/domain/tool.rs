//! MCP tool definition value object.

use super::ToolRegistryDomainError;
use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Canonical metadata for a tool exposed by an MCP server.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct McpToolDefinition {
    name: String,
    description: String,
    input_schema: Value,
    output_schema: Option<Value>,
}

impl McpToolDefinition {
    /// Creates a tool definition with required fields.
    ///
    /// # Errors
    ///
    /// Returns [`ToolRegistryDomainError`] when name or description is empty.
    pub fn new(
        name: impl Into<String>,
        description: impl Into<String>,
        input_schema: Value,
    ) -> Result<Self, ToolRegistryDomainError> {
        let normalized_name = name.into().trim().to_owned();
        if normalized_name.is_empty() {
            return Err(ToolRegistryDomainError::EmptyToolName);
        }

        let normalized_description = description.into().trim().to_owned();
        if normalized_description.is_empty() {
            return Err(ToolRegistryDomainError::EmptyToolDescription);
        }

        Ok(Self {
            name: normalized_name,
            description: normalized_description,
            input_schema,
            output_schema: None,
        })
    }

    /// Sets an optional output schema.
    #[must_use]
    pub fn with_output_schema(mut self, output_schema: Value) -> Self {
        self.output_schema = Some(output_schema);
        self
    }

    /// Returns the tool name.
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Returns the tool description.
    #[must_use]
    pub fn description(&self) -> &str {
        &self.description
    }

    /// Returns the input schema.
    #[must_use]
    pub const fn input_schema(&self) -> &Value {
        &self.input_schema
    }

    /// Returns the optional output schema.
    #[must_use]
    pub const fn output_schema(&self) -> Option<&Value> {
        self.output_schema.as_ref()
    }
}
