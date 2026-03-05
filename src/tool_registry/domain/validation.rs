//! Lightweight JSON schema validation for tool call parameters.
//!
//! Validates that parameters satisfy the basic structural contract
//! declared by a tool's input schema: object type check and required
//! field presence. This avoids a full `jsonschema` crate dependency
//! while covering the common MCP tool schema patterns.

use super::ToolRegistryDomainError;
use serde_json::Value;

/// Validates `parameters` against the structural requirements of
/// `input_schema`.
///
/// Checks performed:
/// 1. If `input_schema` declares `"type": "object"`, `parameters` must
///    be a JSON object.
/// 2. If `input_schema` declares a `"required"` array, every listed key
///    must be present as a top-level key in `parameters`.
///
/// # Errors
///
/// Returns [`ToolRegistryDomainError::SchemaValidationFailed`] when
/// validation fails, with the tool name set to an empty string. The
/// caller should enrich the error with the actual tool name.
pub fn validate_parameters(
    input_schema: &Value,
    parameters: &Value,
) -> Result<(), ToolRegistryDomainError> {
    if input_schema
        .get("type")
        .and_then(Value::as_str)
        .is_some_and(|t| t == "object")
        && !parameters.is_object()
    {
        return Err(ToolRegistryDomainError::SchemaValidationFailed {
            tool_name: String::new(),
            reason: "parameters must be a JSON object".to_owned(),
        });
    }

    check_required_fields(input_schema, parameters)?;

    Ok(())
}

/// Checks that all fields listed in the schema's `"required"` array are
/// present in `parameters`.
fn check_required_fields(
    input_schema: &Value,
    parameters: &Value,
) -> Result<(), ToolRegistryDomainError> {
    let Some(required) = input_schema.get("required").and_then(Value::as_array) else {
        return Ok(());
    };

    let params_obj = parameters.as_object();
    for required_key in required {
        if let Some(key_name) = required_key.as_str()
            && !params_obj.is_some_and(|obj| obj.contains_key(key_name))
        {
            return Err(ToolRegistryDomainError::SchemaValidationFailed {
                tool_name: String::new(),
                reason: format!("missing required parameter: {key_name}"),
            });
        }
    }

    Ok(())
}
