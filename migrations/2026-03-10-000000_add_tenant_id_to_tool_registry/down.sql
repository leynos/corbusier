-- Revert tenant_id additions to tool registry tables.

DROP INDEX idx_mcp_tool_catalog_tenant_tool_name;
DROP INDEX idx_tool_call_audit_log_tenant_id;
DROP INDEX idx_tool_log_metadata_tenant_id;

-- Remove duplicate tool_name rows that may exist across tenants,
-- keeping the earliest-discovered row per tool_name (break ties by ctid).
WITH ranked AS (
    SELECT ctid,
           ROW_NUMBER() OVER (
               PARTITION BY tool_name
               ORDER BY discovered_at ASC, ctid ASC
           ) AS row_num
    FROM mcp_tool_catalog
)
DELETE FROM mcp_tool_catalog
WHERE ctid IN (SELECT ctid FROM ranked WHERE row_num > 1);

CREATE INDEX idx_mcp_tool_catalog_tool_name
    ON mcp_tool_catalog (tool_name);

ALTER TABLE mcp_tool_catalog DROP COLUMN tenant_id;
ALTER TABLE tool_call_audit_log DROP COLUMN tenant_id;
ALTER TABLE tool_log_metadata DROP COLUMN tenant_id;
