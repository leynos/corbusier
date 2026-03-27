//! Metadata summarisation helpers for hook-backed tool governance.

use crate::tool_registry::domain::{
    CatalogEntry, ToolCallOutcome, ToolCallRequest, ToolCallResult,
};
use serde_json::json;

pub(super) fn build_scope_metadata(
    request: &ToolCallRequest,
    entry: &CatalogEntry,
    result: Option<&ToolCallResult>,
) -> serde_json::Value {
    json!({
        "call_id": request.call_id().to_string(),
        "tool_name": request.tool_name(),
        "server_id": entry.server_id().to_string(),
        "parameters": summarize_json_footprint(request.parameters(), 0),
        "workflow_metadata": summarize_json_footprint(request.execution_scope().metadata(), 0),
        "result": result.map_or(serde_json::Value::Null, summarize_result),
    })
}

fn summarize_result(result: &ToolCallResult) -> serde_json::Value {
    match result.outcome() {
        ToolCallOutcome::Success { content } => json!({
            "status": "success",
            "content": summarize_json_footprint(content, 0),
        }),
        ToolCallOutcome::Failure { error } => json!({
            "status": "failure",
            "error": {
                "length": error.chars().count(),
            },
        }),
    }
}

fn summarize_json_footprint(payload: &serde_json::Value, depth: usize) -> serde_json::Value {
    const MAX_FIELDS: usize = 8;
    const MAX_ITEMS: usize = 5;
    const MAX_DEPTH: usize = 3;

    if depth > MAX_DEPTH {
        return json!({
            "kind": "truncated",
            "reason": "max_depth_exceeded",
        });
    }

    match payload {
        serde_json::Value::Null => json!({ "kind": "null" }),
        serde_json::Value::Bool(_) => json!({ "kind": "boolean" }),
        serde_json::Value::Number(_) => json!({ "kind": "number" }),
        serde_json::Value::String(text) => json!({
            "kind": "string",
            "length": text.chars().count(),
        }),
        serde_json::Value::Array(items) => json!({
            "kind": "array",
            "length": items.len(),
            "sample": items
                .iter()
                .take(MAX_ITEMS)
                .map(|item| summarize_json_footprint(item, depth + 1))
                .collect::<Vec<_>>(),
        }),
        serde_json::Value::Object(map) => json!({
            "kind": "object",
            "field_count": map.len(),
            "fields": map
                .iter()
                .take(MAX_FIELDS)
                .map(|(key, field_value)| {
                    (key.clone(), summarize_json_footprint(field_value, depth + 1))
                })
                .collect::<serde_json::Map<_, _>>(),
        }),
    }
}
