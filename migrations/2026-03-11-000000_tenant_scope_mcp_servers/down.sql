BEGIN;

ALTER TABLE mcp_tool_catalog
    DROP CONSTRAINT IF EXISTS mcp_tool_catalog_server_fk;

ALTER TABLE tool_log_metadata
    DROP CONSTRAINT IF EXISTS tool_log_metadata_server_fk;

ALTER TABLE tool_call_audit_log
    DROP CONSTRAINT IF EXISTS tool_call_audit_log_server_fk;

ALTER TABLE mcp_tool_catalog
    ADD CONSTRAINT mcp_tool_catalog_server_id_fkey
        FOREIGN KEY (server_id)
        REFERENCES mcp_servers (id)
        ON DELETE CASCADE;

ALTER TABLE tool_log_metadata
    ADD CONSTRAINT tool_log_metadata_server_id_fkey
        FOREIGN KEY (server_id)
        REFERENCES mcp_servers (id)
        ON DELETE CASCADE;

ALTER TABLE tool_call_audit_log
    ADD CONSTRAINT tool_call_audit_log_server_id_fkey
        FOREIGN KEY (server_id)
        REFERENCES mcp_servers (id)
        ON DELETE CASCADE;

DROP INDEX IF EXISTS idx_tool_call_audit_log_server_tenant;
DROP INDEX IF EXISTS idx_tool_log_metadata_server_tenant;
DROP INDEX IF EXISTS idx_mcp_tool_catalog_server_tenant;
DROP INDEX IF EXISTS idx_mcp_servers_id_tenant;
DROP INDEX IF EXISTS idx_mcp_servers_tenant_name;

ALTER TABLE mcp_servers
    DROP COLUMN tenant_id;

CREATE UNIQUE INDEX idx_mcp_servers_name
    ON mcp_servers (name);

COMMIT;
