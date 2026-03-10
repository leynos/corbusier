//! Stderr log capture and retention domain types.
//!
//! These types model the metadata for captured stderr output from MCP
//! server startup and tool call invocations, along with a configurable
//! retention policy governing log rotation and expiry.

use super::McpServerId;
use super::routing::ToolCallId;
use crate::context::TenantId;
use chrono::{DateTime, Utc};
use mockable::Clock;
use std::fmt;
use uuid::Uuid;

/// Default maximum bytes per individual log blob (10 MiB).
const DEFAULT_MAX_BYTES_PER_LOG: u64 = 10 * 1024 * 1024;

/// Default maximum number of retained logs per server.
const DEFAULT_MAX_LOGS_PER_SERVER: usize = 100;

/// Default retention period in days.
const DEFAULT_RETENTION_DAYS: i64 = 7;

/// Unique identifier for a captured log entry.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct LogEntryId(Uuid);

impl LogEntryId {
    /// Creates a new random log entry identifier.
    #[must_use]
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }

    /// Creates a log entry identifier from an existing UUID.
    #[must_use]
    pub const fn from_uuid(uuid: Uuid) -> Self {
        Self(uuid)
    }

    /// Returns the wrapped UUID.
    #[must_use]
    pub const fn into_inner(self) -> Uuid {
        self.0
    }
}

impl Default for LogEntryId {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Display for LogEntryId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}", self.0)
    }
}

/// Classifies the lifecycle point at which stderr was captured.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LogEntryKind {
    /// Stderr captured during MCP server startup.
    ServerStartup,
    /// Stderr captured during a specific tool call.
    ToolCall {
        /// The tool call that produced this stderr output.
        call_id: ToolCallId,
    },
}

impl LogEntryKind {
    /// Returns the path segment for this kind.
    #[must_use]
    pub fn path_segment(&self) -> String {
        match self {
            Self::ServerStartup => "startup".to_owned(),
            Self::ToolCall { call_id } => format!("call/{call_id}"),
        }
    }
}

/// Bundles the clock, retention policy, and tenant identity used to
/// derive timestamps, expiry, and object paths for log capture
/// operations.
pub struct LogCaptureContext<'a> {
    /// Clock used to obtain the current time.
    pub clock: &'a dyn Clock,
    /// Retention policy governing log expiry.
    pub retention: &'a LogRetentionPolicy,
    /// Tenant that owns this log capture operation.
    pub tenant_id: TenantId,
}

/// Metadata for a captured stderr log blob stored in the object store.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LogEntryMetadata {
    id: LogEntryId,
    server_id: McpServerId,
    kind: LogEntryKind,
    object_path: String,
    byte_count: u64,
    captured_at: DateTime<Utc>,
    expires_at: DateTime<Utc>,
}

impl LogEntryMetadata {
    /// Shared construction logic for all log entry kinds.
    fn build(
        server_id: McpServerId,
        kind: LogEntryKind,
        byte_count: u64,
        ctx: &LogCaptureContext<'_>,
    ) -> Self {
        let id = LogEntryId::new();
        let tenant_id = ctx.tenant_id;
        let object_path = format!(
            "tool_logs/{tenant_id}/{server_id}/{}/{id}.stderr",
            kind.path_segment()
        );
        let captured_at = ctx.clock.utc();
        let expires_at = captured_at + ctx.retention.retention_period;
        Self {
            id,
            server_id,
            kind,
            object_path,
            byte_count,
            captured_at,
            expires_at,
        }
    }

    /// Creates metadata for a server startup stderr capture.
    #[must_use]
    pub fn for_startup(
        server_id: McpServerId,
        byte_count: u64,
        ctx: &LogCaptureContext<'_>,
    ) -> Self {
        Self::build(server_id, LogEntryKind::ServerStartup, byte_count, ctx)
    }

    /// Creates metadata for a tool call stderr capture.
    #[must_use]
    pub fn for_tool_call(
        server_id: McpServerId,
        call_id: ToolCallId,
        byte_count: u64,
        ctx: &LogCaptureContext<'_>,
    ) -> Self {
        Self::build(
            server_id,
            LogEntryKind::ToolCall { call_id },
            byte_count,
            ctx,
        )
    }

    /// Returns the log entry identifier.
    #[must_use]
    pub const fn id(&self) -> LogEntryId {
        self.id
    }

    /// Returns the server that produced this log.
    #[must_use]
    pub const fn server_id(&self) -> McpServerId {
        self.server_id
    }

    /// Returns the lifecycle point classification.
    #[must_use]
    pub const fn kind(&self) -> &LogEntryKind {
        &self.kind
    }

    /// Returns the object store path where the log blob is stored.
    #[must_use]
    pub fn object_path(&self) -> &str {
        &self.object_path
    }

    /// Returns the size of the captured log in bytes.
    #[must_use]
    pub const fn byte_count(&self) -> u64 {
        self.byte_count
    }

    /// Returns the capture timestamp.
    #[must_use]
    pub const fn captured_at(&self) -> DateTime<Utc> {
        self.captured_at
    }

    /// Returns the expiry timestamp.
    #[must_use]
    pub const fn expires_at(&self) -> DateTime<Utc> {
        self.expires_at
    }
}

/// Configurable policy for stderr log rotation and retention.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LogRetentionPolicy {
    /// Maximum size in bytes for a single log blob before truncation.
    pub max_bytes_per_log: u64,
    /// Maximum number of retained log entries per server.
    pub max_logs_per_server: usize,
    /// Duration after which log entries expire and are eligible for
    /// deletion.
    pub retention_period: chrono::Duration,
}

impl Default for LogRetentionPolicy {
    fn default() -> Self {
        Self {
            max_bytes_per_log: DEFAULT_MAX_BYTES_PER_LOG,
            max_logs_per_server: DEFAULT_MAX_LOGS_PER_SERVER,
            retention_period: chrono::Duration::days(DEFAULT_RETENTION_DAYS),
        }
    }
}

impl LogRetentionPolicy {
    /// Returns whether the given log entry has expired.
    #[must_use]
    pub fn is_expired(&self, entry: &LogEntryMetadata, now: DateTime<Utc>) -> bool {
        entry.expires_at < now
    }
}
