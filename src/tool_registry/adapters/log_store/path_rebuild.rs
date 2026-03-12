//! Helpers for rebuilding log metadata from object-store paths.

use crate::tool_registry::domain::{
    LogEntryId, LogEntryKind, LogEntryMetadata, McpServerId, PersistedLogEntryData, ToolCallId,
};
use object_store::ObjectMeta;
use tracing::warn;
use uuid::Uuid;

pub(super) fn rebuild_metadata_from_object_meta(
    meta: &ObjectMeta,
    server_id: McpServerId,
    retention_period: chrono::Duration,
) -> Option<LogEntryMetadata> {
    let path = meta.location.to_string();
    if path.is_empty() {
        warn_rebuild_metadata_failure(meta, "empty object-store path");
        return None;
    }
    let segments: Vec<&str> = path.split('/').collect();
    let file_name = extract_file_name(meta, &segments)?;
    let id = parse_log_entry_id(meta, file_name)?;
    let kind = parse_log_kind(meta, &segments)?;
    let byte_count = meta.size;
    let captured_at = meta.last_modified;
    let expires_at = captured_at + retention_period;
    Some(LogEntryMetadata::from_persisted(PersistedLogEntryData {
        id,
        server_id,
        kind,
        object_path: path,
        byte_count,
        captured_at,
        expires_at,
    }))
}

fn extract_file_name<'a>(meta: &ObjectMeta, segments: &'a [&str]) -> Option<&'a str> {
    let Some(file_name) = segments.last().copied() else {
        warn_rebuild_metadata_failure(meta, "missing file name segment");
        return None;
    };
    if file_name.is_empty() {
        warn_rebuild_metadata_failure(meta, "empty file name segment");
        return None;
    }
    Some(file_name)
}

fn parse_log_entry_id(meta: &ObjectMeta, file_name: &str) -> Option<LogEntryId> {
    let Some(id_segment) = file_name.strip_suffix(".stderr") else {
        warn_rebuild_metadata_failure(meta, "missing .stderr suffix");
        return None;
    };
    if id_segment.is_empty() {
        warn_rebuild_metadata_failure(meta, "empty log entry id segment");
        return None;
    }
    match Uuid::parse_str(id_segment) {
        Ok(uuid) => Some(LogEntryId::from_uuid(uuid)),
        Err(err) => {
            warn_rebuild_metadata_failure_with_error(
                meta,
                "invalid log entry id",
                &err.to_string(),
            );
            None
        }
    }
}

fn parse_log_kind(meta: &ObjectMeta, segments: &[&str]) -> Option<LogEntryKind> {
    match segments {
        ["tool_logs", _, _, "startup", _] => Some(LogEntryKind::ServerStartup),
        ["tool_logs", _, _, "call", call_id, _] => match Uuid::parse_str(call_id) {
            Ok(uuid) => Some(LogEntryKind::ToolCall {
                call_id: ToolCallId::from_uuid(uuid),
            }),
            Err(err) => {
                warn_rebuild_metadata_failure_with_error(
                    meta,
                    "invalid tool call id",
                    &err.to_string(),
                );
                None
            }
        },
        _ => {
            warn_rebuild_metadata_failure(meta, "unrecognised log path shape");
            None
        }
    }
}

fn warn_rebuild_metadata_failure(meta: &ObjectMeta, reason: &str) {
    warn!(
        function = "rebuild_metadata_from_object_meta",
        location = %meta.location,
        reason,
    );
}

fn warn_rebuild_metadata_failure_with_error(meta: &ObjectMeta, reason: &str, error_message: &str) {
    warn!(
        function = "rebuild_metadata_from_object_meta",
        location = %meta.location,
        reason,
        error = error_message,
    );
}
