-- Revert tenant_id additions to tool registry tables.

DROP INDEX idx_mcp_tool_catalog_tenant_tool_name;

CREATE UNIQUE INDEX idx_mcp_tool_catalog_tool_name
    ON mcp_tool_catalog (tool_name);

ALTER TABLE mcp_tool_catalog DROP COLUMN tenant_id;
ALTER TABLE tool_call_audit_log DROP COLUMN tenant_id;
ALTER TABLE tool_log_metadata DROP COLUMN tenant_id;
