//! Then steps for tool discovery and routing BDD scenarios.

use corbusier::tool_registry::{
    domain::{ToolCallRequest, ToolRegistryDomainError},
    services::ToolDiscoveryRoutingServiceError,
};
use eyre::{WrapErr, eyre};
use mockable::DefaultClock;
use rstest_bdd_macros::then;
use serde_json::json;

use super::run_async;
use super::world::ToolDiscoveryWorld;

#[then(r"the tool catalogue contains {count:usize} entry")]
fn catalog_contains_count(
    world: &mut ToolDiscoveryWorld,
    count: usize,
) -> Result<(), eyre::Report> {
    let entries = run_async(world.discovery()?.list_catalog(&world.request_ctx))
        .wrap_err("catalogue listing should succeed")?;
    if entries.len() != count {
        return Err(eyre!(
            "expected {count} catalogue entries, got {}",
            entries.len()
        ));
    }
    Ok(())
}

#[then(r#"tool "{tool_name}" is marked as available"#)]
fn tool_is_available(
    world: &mut ToolDiscoveryWorld,
    tool_name: String,
) -> Result<(), eyre::Report> {
    let entries = run_async(world.discovery()?.list_catalog(&world.request_ctx))
        .wrap_err("catalogue listing should succeed")?;
    let entry = entries
        .iter()
        .find(|e| e.tool().name() == tool_name)
        .ok_or_else(|| eyre!("tool '{tool_name}' not found in catalogue"))?;
    if !entry.available() {
        return Err(eyre!("tool '{tool_name}' should be available but is not"));
    }
    Ok(())
}

#[then("the tool call succeeds")]
fn tool_call_succeeds(world: &ToolDiscoveryWorld) -> Result<(), eyre::Report> {
    match world.last_call_succeeded {
        Some(true) => Ok(()),
        Some(false) => Err(eyre!(
            "tool call should have succeeded but failed: {:?}",
            world.last_error
        )),
        None => Err(eyre!("no tool call was made")),
    }
}

#[then(r#"the audit log contains {count:usize} entry for tool "{tool_name}""#)]
fn audit_log_contains_entry(
    world: &ToolDiscoveryWorld,
    count: usize,
    tool_name: String,
) -> Result<(), eyre::Report> {
    let records = world
        .catalog
        .audit_records()
        .map_err(|err| eyre!("audit records retrieval failed: {err}"))?;
    let matching: Vec<_> = records
        .iter()
        .filter(|r| r.tool_name() == tool_name)
        .collect();
    if matching.len() != count {
        return Err(eyre!(
            "expected {count} audit entries for '{tool_name}', got {}",
            matching.len()
        ));
    }
    Ok(())
}

#[then(r#"calling tool "{tool_name}" is rejected as unavailable"#)]
fn tool_call_rejected_unavailable(
    world: &mut ToolDiscoveryWorld,
    tool_name: String,
) -> Result<(), eyre::Report> {
    let request = ToolCallRequest::new(&tool_name, json!({"path": "/tmp/test.txt"}), &DefaultClock);
    let result = run_async(world.discovery()?.call_tool(&world.request_ctx, &request));
    if !matches!(
        result,
        Err(ToolDiscoveryRoutingServiceError::Domain(
            ToolRegistryDomainError::ToolUnavailable { .. }
        ))
    ) {
        return Err(eyre!("expected ToolUnavailable error, got {result:?}"));
    }
    Ok(())
}

#[then(r#"calling tool "{tool_name}" is rejected as not found"#)]
fn tool_call_rejected_not_found(
    world: &mut ToolDiscoveryWorld,
    tool_name: String,
) -> Result<(), eyre::Report> {
    let request = ToolCallRequest::new(&tool_name, json!({}), &DefaultClock);
    let result = run_async(world.discovery()?.call_tool(&world.request_ctx, &request));
    if !matches!(
        result,
        Err(ToolDiscoveryRoutingServiceError::Domain(
            ToolRegistryDomainError::ToolNotFound(_)
        ))
    ) {
        return Err(eyre!("expected ToolNotFound error, got {result:?}"));
    }
    Ok(())
}

#[then(r#"the audit log entry for tool "{tool_name}" has a stderr log path"#)]
fn audit_entry_has_stderr_log_path(
    world: &ToolDiscoveryWorld,
    tool_name: String,
) -> Result<(), eyre::Report> {
    let records = world
        .catalog
        .audit_records()
        .map_err(|err| eyre!("audit records retrieval failed: {err}"))?;
    let entry = records
        .iter()
        .find(|r| r.tool_name() == tool_name)
        .ok_or_else(|| eyre!("no audit entry for tool '{tool_name}'"))?;
    if entry.stderr_log_path().is_none() {
        return Err(eyre!(
            "audit entry for '{tool_name}' should have a stderr log path"
        ));
    }
    Ok(())
}
