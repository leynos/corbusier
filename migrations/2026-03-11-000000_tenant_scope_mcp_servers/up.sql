BEGIN;

-- Tenant-scope MCP servers and enforce tenant-consistent parent/child links.
ALTER TABLE mcp_servers
    ADD COLUMN tenant_id UUID NOT NULL
        DEFAULT '00000000-0000-0000-0000-000000000001';

ALTER TABLE mcp_servers
    ALTER COLUMN tenant_id DROP DEFAULT;

DROP INDEX IF EXISTS idx_mcp_servers_name;

CREATE UNIQUE INDEX idx_mcp_servers_tenant_name
    ON mcp_servers (tenant_id, name);

CREATE UNIQUE INDEX idx_mcp_servers_id_tenant
    ON mcp_servers (id, tenant_id);

ALTER TABLE mcp_tool_catalog
    DROP CONSTRAINT IF EXISTS mcp_tool_catalog_server_id_fkey;

ALTER TABLE tool_log_metadata
    DROP CONSTRAINT IF EXISTS tool_log_metadata_server_id_fkey;

ALTER TABLE tool_call_audit_log
    DROP CONSTRAINT IF EXISTS tool_call_audit_log_server_id_fkey;

ALTER TABLE mcp_tool_catalog
    ADD CONSTRAINT mcp_tool_catalog_server_fk
        FOREIGN KEY (server_id, tenant_id)
        REFERENCES mcp_servers (id, tenant_id)
        ON DELETE CASCADE;

ALTER TABLE tool_log_metadata
    ADD CONSTRAINT tool_log_metadata_server_fk
        FOREIGN KEY (server_id, tenant_id)
        REFERENCES mcp_servers (id, tenant_id)
        ON DELETE CASCADE;

ALTER TABLE tool_call_audit_log
    ADD CONSTRAINT tool_call_audit_log_server_fk
        FOREIGN KEY (server_id, tenant_id)
        REFERENCES mcp_servers (id, tenant_id)
        ON DELETE CASCADE;

CREATE INDEX IF NOT EXISTS idx_mcp_tool_catalog_server_tenant
    ON mcp_tool_catalog (tenant_id, server_id);

CREATE INDEX IF NOT EXISTS idx_tool_log_metadata_server_tenant
    ON tool_log_metadata (tenant_id, server_id);

CREATE INDEX IF NOT EXISTS idx_tool_call_audit_log_server_tenant
    ON tool_call_audit_log (tenant_id, server_id);

COMMIT;
