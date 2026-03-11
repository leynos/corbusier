//! Audit-row mapping helpers for the tool catalog `PostgreSQL` adapter.

use crate::tool_registry::{
    adapters::postgres::catalog_models::NewAuditLogRow,
    domain::{
        ToolCallAuditRecord, ToolCallOutcome, redact_error_message, redact_outcome_content,
        redact_parameters,
    },
};

/// Maps a tool-call audit record into an insertable row.
#[expect(
    clippy::cast_possible_truncation,
    reason = "duration_ms is always positive and within i64 range for tool calls"
)]
pub(super) fn audit_to_new_row(
    record: &ToolCallAuditRecord,
    tenant_id: uuid::Uuid,
) -> NewAuditLogRow {
    let (outcome_str, outcome_content, outcome_error) = match record.outcome() {
        ToolCallOutcome::Success { content } => (
            "success".to_owned(),
            Some(redact_outcome_content(content)),
            None,
        ),
        ToolCallOutcome::Failure { error } => (
            "failure".to_owned(),
            None,
            Some(redact_error_message(error)),
        ),
    };

    NewAuditLogRow {
        id: record.id(),
        tenant_id,
        call_id: record.call_id().into_inner(),
        tool_name: record.tool_name().to_owned(),
        server_id: record.server_id().into_inner(),
        parameters: redact_parameters(record.parameters()),
        outcome: outcome_str,
        outcome_content,
        outcome_error,
        duration_ms: record.duration().as_millis() as i64,
        initiated_at: record.initiated_at(),
        completed_at: record.completed_at(),
        stderr_log_path: record.stderr_log_path().map(str::to_owned),
    }
}
