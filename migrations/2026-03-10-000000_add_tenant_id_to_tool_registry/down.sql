-- Revert tenant_id additions to tool registry tables.
--
-- Guard: if multiple tenants registered the same tool_name, the global
-- unique index would fail.  Delete duplicates keeping the earliest entry.

DROP INDEX idx_mcp_tool_catalog_tenant_tool_name;
DROP INDEX idx_tool_call_audit_log_tenant_id;
DROP INDEX idx_tool_log_metadata_tenant_id;

DELETE FROM mcp_tool_catalog a
    USING mcp_tool_catalog b
    WHERE a.tool_name = b.tool_name
      AND a.discovered_at > b.discovered_at;

CREATE UNIQUE INDEX idx_mcp_tool_catalog_tool_name
    ON mcp_tool_catalog (tool_name);

ALTER TABLE mcp_tool_catalog DROP COLUMN tenant_id;
ALTER TABLE tool_call_audit_log DROP COLUMN tenant_id;
ALTER TABLE tool_log_metadata DROP COLUMN tenant_id;
