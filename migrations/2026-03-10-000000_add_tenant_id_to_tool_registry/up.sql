-- Add tenant_id to tool registry tables for tenant isolation.
--
-- NOTE: mcp_servers has no tenant_id column, so we cannot create a
-- composite FK (server_id, tenant_id) referencing mcp_servers.  The FK
-- remains server_id -> mcp_servers(id) only.  This invariant is
-- documented in corbusier-design.md.

-- 1. mcp_tool_catalog
ALTER TABLE mcp_tool_catalog
    ADD COLUMN tenant_id UUID NOT NULL
        DEFAULT '00000000-0000-0000-0000-000000000000';

ALTER TABLE mcp_tool_catalog
    ALTER COLUMN tenant_id DROP DEFAULT;

DROP INDEX idx_mcp_tool_catalog_tool_name;

CREATE UNIQUE INDEX idx_mcp_tool_catalog_tenant_tool_name
    ON mcp_tool_catalog (tenant_id, tool_name);

-- 2. tool_call_audit_log
ALTER TABLE tool_call_audit_log
    ADD COLUMN tenant_id UUID NOT NULL
        DEFAULT '00000000-0000-0000-0000-000000000000';

ALTER TABLE tool_call_audit_log
    ALTER COLUMN tenant_id DROP DEFAULT;

CREATE INDEX idx_tool_call_audit_log_tenant_id
    ON tool_call_audit_log (tenant_id);

-- 3. tool_log_metadata
ALTER TABLE tool_log_metadata
    ADD COLUMN tenant_id UUID NOT NULL
        DEFAULT '00000000-0000-0000-0000-000000000000';

ALTER TABLE tool_log_metadata
    ALTER COLUMN tenant_id DROP DEFAULT;

CREATE INDEX idx_tool_log_metadata_tenant_id
    ON tool_log_metadata (tenant_id);
