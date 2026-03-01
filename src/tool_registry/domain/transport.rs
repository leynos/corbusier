//! MCP server transport configuration value objects.

use super::ToolRegistryDomainError;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// Transport settings for an MCP server hosted over STDIO.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StdioTransportConfig {
    command: String,
    args: Vec<String>,
    env: BTreeMap<String, String>,
    working_directory: Option<String>,
}

impl StdioTransportConfig {
    /// Creates a new STDIO transport configuration.
    ///
    /// # Errors
    ///
    /// Returns [`ToolRegistryDomainError::EmptyStdioCommand`] when `command`
    /// is empty after trimming.
    pub fn new(command: impl Into<String>) -> Result<Self, ToolRegistryDomainError> {
        let normalized_command = command.into().trim().to_owned();
        if normalized_command.is_empty() {
            return Err(ToolRegistryDomainError::EmptyStdioCommand);
        }

        Ok(Self {
            command: normalized_command,
            args: Vec::new(),
            env: BTreeMap::new(),
            working_directory: None,
        })
    }

    /// Appends command-line arguments.
    #[must_use]
    pub fn with_args(mut self, values: impl IntoIterator<Item = String>) -> Self {
        self.args = values.into_iter().collect();
        self
    }

    /// Replaces process environment variables.
    #[must_use]
    pub fn with_env(mut self, values: impl IntoIterator<Item = (String, String)>) -> Self {
        self.env = values.into_iter().collect();
        self
    }

    /// Sets an explicit working directory.
    ///
    /// # Errors
    ///
    /// Returns [`ToolRegistryDomainError::EmptyWorkingDirectory`] when the
    /// provided value is empty after trimming.
    pub fn with_working_directory(
        mut self,
        value: impl Into<String>,
    ) -> Result<Self, ToolRegistryDomainError> {
        let normalized = value.into().trim().to_owned();
        if normalized.is_empty() {
            return Err(ToolRegistryDomainError::EmptyWorkingDirectory);
        }

        self.working_directory = Some(normalized);
        Ok(self)
    }

    /// Returns the executable command.
    #[must_use]
    pub fn command(&self) -> &str {
        &self.command
    }

    /// Returns command-line arguments.
    #[must_use]
    pub fn args(&self) -> &[String] {
        &self.args
    }

    /// Returns environment variables.
    #[must_use]
    pub const fn env(&self) -> &BTreeMap<String, String> {
        &self.env
    }

    /// Returns the optional working directory.
    #[must_use]
    pub fn working_directory(&self) -> Option<&str> {
        self.working_directory.as_deref()
    }
}

/// Transport settings for an MCP server hosted over HTTP+SSE.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HttpSseTransportConfig {
    base_url: String,
}

impl HttpSseTransportConfig {
    /// Creates a new HTTP+SSE transport configuration.
    ///
    /// # Errors
    ///
    /// Returns [`ToolRegistryDomainError`] when `base_url` is empty or does
    /// not start with `http://` or `https://`.
    pub fn new(base_url: impl Into<String>) -> Result<Self, ToolRegistryDomainError> {
        let normalized_base_url = base_url.into().trim().to_owned();
        if normalized_base_url.is_empty() {
            return Err(ToolRegistryDomainError::EmptyHttpSseBaseUrl);
        }

        let has_valid_prefix = normalized_base_url.starts_with("http://")
            || normalized_base_url.starts_with("https://");
        if !has_valid_prefix {
            return Err(ToolRegistryDomainError::InvalidHttpSseBaseUrl(
                normalized_base_url,
            ));
        }

        Ok(Self {
            base_url: normalized_base_url,
        })
    }

    /// Returns the base URL.
    #[must_use]
    pub fn base_url(&self) -> &str {
        &self.base_url
    }
}

/// Supported MCP transport configuration variants.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "kind", content = "config")]
pub enum McpTransport {
    /// MCP over local process STDIO.
    Stdio(StdioTransportConfig),
    /// MCP over HTTP+SSE.
    HttpSse(HttpSseTransportConfig),
}

impl McpTransport {
    /// Creates a `stdio` transport.
    ///
    /// # Errors
    ///
    /// Returns validation errors from [`StdioTransportConfig::new`].
    pub fn stdio(command: impl Into<String>) -> Result<Self, ToolRegistryDomainError> {
        Ok(Self::Stdio(StdioTransportConfig::new(command)?))
    }

    /// Creates an `http_sse` transport.
    ///
    /// # Errors
    ///
    /// Returns validation errors from [`HttpSseTransportConfig::new`].
    pub fn http_sse(base_url: impl Into<String>) -> Result<Self, ToolRegistryDomainError> {
        Ok(Self::HttpSse(HttpSseTransportConfig::new(base_url)?))
    }
}
