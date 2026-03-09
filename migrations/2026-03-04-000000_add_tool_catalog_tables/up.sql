-- Tool catalog and audit trail tables for roadmap 2.1.2.
-- Follows corbusier-design.md §2.2.4 (F-005-RQ-002, F-005-RQ-003).

-- Tool catalog: durable index of tools discovered from MCP servers.
CREATE TABLE mcp_tool_catalog (
    id UUID PRIMARY KEY,
    server_id UUID NOT NULL REFERENCES mcp_servers(id) ON DELETE CASCADE,
    server_name VARCHAR(100) NOT NULL,
    tool_name VARCHAR(255) NOT NULL,
    tool_description TEXT NOT NULL,
    input_schema JSONB NOT NULL,
    output_schema JSONB,
    available BOOLEAN NOT NULL DEFAULT true,
    discovered_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Unique tool name constraint: routing requires unambiguous resolution.
CREATE UNIQUE INDEX idx_mcp_tool_catalog_tool_name ON mcp_tool_catalog (tool_name);
CREATE INDEX idx_mcp_tool_catalog_server_id ON mcp_tool_catalog (server_id);
CREATE INDEX idx_mcp_tool_catalog_available_tool ON mcp_tool_catalog (available, tool_name);

-- Auto-update trigger for updated_at.
CREATE FUNCTION update_mcp_tool_catalog_updated_at()
RETURNS TRIGGER AS $$
BEGIN
    NEW.updated_at = NOW();
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER trg_mcp_tool_catalog_updated_at
    BEFORE UPDATE ON mcp_tool_catalog
    FOR EACH ROW
    EXECUTE FUNCTION update_mcp_tool_catalog_updated_at();

-- Tool call audit log: immutable record of every tool invocation.
CREATE TABLE tool_call_audit_log (
    id UUID PRIMARY KEY,
    call_id UUID NOT NULL,
    tool_name VARCHAR(255) NOT NULL,
    server_id UUID NOT NULL,
    parameters JSONB NOT NULL,
    outcome VARCHAR(50) NOT NULL,
    outcome_content JSONB,
    outcome_error TEXT,
    duration_ms BIGINT NOT NULL,
    initiated_at TIMESTAMPTZ NOT NULL,
    completed_at TIMESTAMPTZ NOT NULL,
    stderr_log_path VARCHAR(512),
    CONSTRAINT tool_call_audit_log_outcome_check CHECK (
        outcome IN ('success', 'failure')
    ),
    CONSTRAINT tool_call_audit_log_outcome_content_check CHECK (
        (outcome = 'success' AND outcome_content IS NOT NULL AND outcome_error IS NULL)
        OR (outcome = 'failure' AND outcome_error IS NOT NULL AND outcome_content IS NULL)
    )
);

CREATE INDEX idx_tool_call_audit_log_call_id ON tool_call_audit_log (call_id);
CREATE INDEX idx_tool_call_audit_log_tool_name ON tool_call_audit_log (tool_name);
CREATE INDEX idx_tool_call_audit_log_initiated_at ON tool_call_audit_log (initiated_at);

-- Stderr log metadata: index of log blobs stored in object_store.
CREATE TABLE tool_log_metadata (
    id UUID PRIMARY KEY,
    server_id UUID NOT NULL REFERENCES mcp_servers(id) ON DELETE CASCADE,
    kind VARCHAR(50) NOT NULL,
    call_id UUID,
    object_path VARCHAR(512) NOT NULL,
    byte_count BIGINT NOT NULL,
    captured_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    expires_at TIMESTAMPTZ NOT NULL,
    CONSTRAINT tool_log_metadata_kind_check CHECK (
        kind IN ('startup', 'tool_call')
    ),
    CONSTRAINT tool_log_metadata_call_id_check CHECK (
        (kind = 'tool_call' AND call_id IS NOT NULL)
        OR (kind = 'startup' AND call_id IS NULL)
    )
);

CREATE INDEX idx_tool_log_metadata_server_kind ON tool_log_metadata (server_id, kind);
CREATE INDEX idx_tool_log_metadata_expires_at ON tool_log_metadata (expires_at);
CREATE UNIQUE INDEX idx_tool_log_metadata_object_path ON tool_log_metadata (object_path);
