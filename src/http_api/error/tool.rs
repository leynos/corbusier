//! Tool discovery and routing HTTP error mappings.

use super::ApiError;
use crate::tool_registry::{
    domain::ToolRegistryDomainError,
    ports::{
        McpServerHostError, McpServerRegistryError, ToolCatalogError, ToolGovernanceError,
        ToolLogStoreError,
    },
    services::ToolDiscoveryRoutingServiceError,
};
use actix_web::http::StatusCode;

pub(crate) fn map_tool_service_error(error: ToolDiscoveryRoutingServiceError) -> ApiError {
    match error {
        ToolDiscoveryRoutingServiceError::Registry(registry_error) => {
            map_tool_service_infrastructure_error(ToolServiceInfrastructureError::Registry(
                registry_error,
            ))
        }
        ToolDiscoveryRoutingServiceError::Host(host_error) => {
            map_tool_service_infrastructure_error(ToolServiceInfrastructureError::Host(host_error))
        }
        other => map_tool_service_client_error(other),
    }
}

fn map_tool_service_client_error(error: ToolDiscoveryRoutingServiceError) -> ApiError {
    match error {
        ToolDiscoveryRoutingServiceError::Domain(domain_error) => {
            map_tool_domain_error(domain_error)
        }
        ToolDiscoveryRoutingServiceError::Catalog(catalog_error) => {
            map_tool_catalog_error(catalog_error)
        }
        ToolDiscoveryRoutingServiceError::Governance(governance_error) => {
            map_tool_governance_error(&governance_error)
        }
        ToolDiscoveryRoutingServiceError::LogStore(log_store_error) => {
            map_tool_log_store_error(&log_store_error)
        }
        ToolDiscoveryRoutingServiceError::NotFound(server_id) => {
            ApiError::not_found("mcp_server_not_found", server_id.to_string())
        }
        ToolDiscoveryRoutingServiceError::Registry(_)
        | ToolDiscoveryRoutingServiceError::Host(_) => {
            debug_assert!(
                false,
                "infrastructure errors should be handled before client error mapping"
            );
            ApiError::internal()
        }
    }
}

#[derive(Debug)]
enum ToolServiceInfrastructureError {
    Registry(McpServerRegistryError),
    Host(McpServerHostError),
}

fn map_tool_service_infrastructure_error(error: ToolServiceInfrastructureError) -> ApiError {
    match error {
        ToolServiceInfrastructureError::Registry(registry_error) => {
            log_tool_registry_error(&registry_error);
            ApiError::internal()
        }
        ToolServiceInfrastructureError::Host(host_error) => {
            log_tool_host_error(&host_error);
            ApiError::internal()
        }
    }
}

pub(crate) fn map_tool_domain_error(error: ToolRegistryDomainError) -> ApiError {
    match error {
        ToolRegistryDomainError::EmptyServerName
        | ToolRegistryDomainError::InvalidServerName(_)
        | ToolRegistryDomainError::ServerNameTooLong(_)
        | ToolRegistryDomainError::EmptyStdioCommand
        | ToolRegistryDomainError::EmptyWorkingDirectory
        | ToolRegistryDomainError::EmptyHttpSseBaseUrl
        | ToolRegistryDomainError::InvalidHttpSseBaseUrl(_)
        | ToolRegistryDomainError::EmptyToolName
        | ToolRegistryDomainError::EmptyToolDescription => {
            ApiError::bad_request("tool_request_failed", error.to_string())
        }
        ToolRegistryDomainError::InvalidLifecycleTransition { .. } => {
            ApiError::conflict("invalid_mcp_server_transition", error.to_string())
        }
        ToolRegistryDomainError::ToolQueryRequiresRunning { server_id, .. } => {
            ApiError::conflict("mcp_server_not_running", server_id.to_string())
        }
        ToolRegistryDomainError::ToolNotFound(tool_name) => {
            ApiError::not_found("tool_not_found", tool_name)
        }
        ToolRegistryDomainError::ToolUnavailable { tool_name, .. } => {
            ApiError::conflict("tool_unavailable", tool_name)
        }
        ToolRegistryDomainError::PolicyDenied { reason, .. } => {
            ApiError::new(StatusCode::FORBIDDEN, "tool_policy_denied", reason)
        }
        ToolRegistryDomainError::SchemaValidationFailed { reason, .. } => {
            ApiError::bad_request("tool_schema_validation_failed", reason)
        }
        ToolRegistryDomainError::AmbiguousToolName { tool_name, .. } => {
            ApiError::conflict("ambiguous_tool_name", tool_name)
        }
        ToolRegistryDomainError::ToolCallTimeout { tool_name, .. } => {
            ApiError::new(StatusCode::GATEWAY_TIMEOUT, "tool_call_timeout", tool_name)
        }
    }
}

pub(crate) fn map_tool_catalog_error(error: ToolCatalogError) -> ApiError {
    match error {
        ToolCatalogError::DuplicateEntry { tool_name, .. }
        | ToolCatalogError::DuplicateWithinBatch { tool_name, .. } => {
            ApiError::conflict("duplicate_tool_catalog_entry", tool_name)
        }
        ToolCatalogError::NotFound(name) => {
            ApiError::not_found("tool_catalog_entry_not_found", name)
        }
        ToolCatalogError::MixedServerBatch { reason } => {
            ApiError::bad_request("mixed_tool_server_batch", reason)
        }
        ToolCatalogError::InvalidPersistedData { .. } | ToolCatalogError::Persistence { .. } => {
            tracing::error!(error = %error, "tool catalog error");
            ApiError::internal()
        }
    }
}

pub(crate) fn map_tool_governance_error(error: &ToolGovernanceError) -> ApiError {
    tracing::error!(error = %error, "tool governance error");
    ApiError::internal()
}

pub(crate) fn map_tool_log_store_error(error: &ToolLogStoreError) -> ApiError {
    tracing::error!(error = %error, "tool log store error");
    ApiError::internal()
}

pub(crate) fn log_tool_registry_error(error: &McpServerRegistryError) {
    tracing::error!(error = %error, "registry error");
}

pub(crate) fn log_tool_host_error(error: &McpServerHostError) {
    tracing::error!(error = %error, "host error");
}
