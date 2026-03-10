//! Audit payload redaction for sensitive data.
//!
//! Provides functions to redact parameter values and outcome content
//! before persistence in the audit trail, replacing all leaf string
//! values with `"[REDACTED]"`.

use serde_json::Value;

const REDACTED: &str = "[REDACTED]";

/// Deep-clones a JSON value, replacing every string leaf with
/// `"[REDACTED]"`.
///
/// Object keys are preserved; only values are redacted.  Arrays are
/// traversed recursively.  Non-string leaves (numbers, booleans, null)
/// are preserved.
#[must_use]
pub fn redact_parameters(params: &Value) -> Value {
    redact_value(params)
}

/// Redacts outcome content for audit persistence.
///
/// The outcome content is a [`serde_json::Value`] (typically an object
/// or array of content items).  All string leaves are replaced with
/// `"[REDACTED]"`.
#[must_use]
pub fn redact_outcome_content(content: &Value) -> Value {
    redact_value(content)
}

/// Redacts a tool-call error message for audit persistence.
///
/// Error strings may contain file paths, internal identifiers, or
/// other sensitive context.  This replaces the full error text with
/// `"[REDACTED]"` to match the redaction applied to parameters and
/// outcome content.
#[must_use]
pub fn redact_error_message(_error: &str) -> String {
    REDACTED.to_owned()
}

fn redact_value(value: &Value) -> Value {
    match value {
        Value::String(_) => Value::String(REDACTED.to_owned()),
        Value::Array(arr) => Value::Array(arr.iter().map(redact_value).collect()),
        Value::Object(map) => Value::Object(
            map.iter()
                .map(|(k, v)| (k.clone(), redact_value(v)))
                .collect(),
        ),
        other => other.clone(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn replaces_string_values() {
        let params = json!({"path": "/tmp/secret.txt", "mode": "read"});
        let redacted = redact_parameters(&params);
        assert_eq!(
            redacted,
            json!({"path": "[REDACTED]", "mode": "[REDACTED]"})
        );
    }

    #[test]
    fn preserves_non_string_leaves() {
        let params = json!({"count": 42, "flag": true, "empty": null});
        let redacted = redact_parameters(&params);
        assert_eq!(redacted, json!({"count": 42, "flag": true, "empty": null}));
    }

    #[test]
    fn handles_nested_objects() {
        let params = json!({"outer": {"inner": "secret"}});
        let redacted = redact_parameters(&params);
        assert_eq!(redacted, json!({"outer": {"inner": "[REDACTED]"}}));
    }

    #[test]
    fn handles_arrays() {
        let params = json!({"items": ["a", "b", 3]});
        let redacted = redact_parameters(&params);
        assert_eq!(redacted, json!({"items": ["[REDACTED]", "[REDACTED]", 3]}));
    }

    #[test]
    fn redacts_outcome_content_text() {
        let content = json!([{"type": "text", "text": "sensitive output"}]);
        let redacted = redact_outcome_content(&content);
        assert_eq!(
            redacted,
            json!([{"type": "[REDACTED]", "text": "[REDACTED]"}])
        );
    }

    #[test]
    fn handles_empty_object() {
        let params = json!({});
        let redacted = redact_parameters(&params);
        assert_eq!(redacted, json!({}));
    }
}
