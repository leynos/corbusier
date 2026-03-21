//! Hook execution log level and entry types.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Log severity level for hook execution.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HookLogLevel {
    /// Informational log entry.
    Info,
    /// Warning log entry.
    Warning,
    /// Error log entry.
    Error,
}

/// Structured log entry emitted during hook execution.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HookLogEntry {
    level: HookLogLevel,
    message: String,
    timestamp: DateTime<Utc>,
}

impl HookLogEntry {
    /// Creates a new log entry.
    /// Example: `HookLogEntry::new(HookLogLevel::Info, "ok", timestamp)` creates an info log entry.
    #[must_use]
    pub fn new(level: HookLogLevel, message: impl Into<String>, timestamp: DateTime<Utc>) -> Self {
        Self {
            level,
            message: message.into(),
            timestamp,
        }
    }

    /// Returns the log level. Example: `entry.level()` returns `HookLogLevel::Info`.
    #[must_use]
    pub const fn level(&self) -> HookLogLevel {
        self.level
    }

    /// Returns the log message. Example: `entry.message()` returns the log text.
    #[must_use]
    pub fn message(&self) -> &str {
        &self.message
    }

    /// Returns the log timestamp. Example: `entry.timestamp()` returns the timestamp value.
    #[must_use]
    pub const fn timestamp(&self) -> DateTime<Utc> {
        self.timestamp
    }
}
