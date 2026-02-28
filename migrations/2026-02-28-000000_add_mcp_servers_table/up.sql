-- Add MCP server registry table for roadmap item 2.1.1.
-- Follows corbusier-design.md §2.2.4 and §6.1.4.

CREATE TABLE mcp_servers (
    id UUID PRIMARY KEY,
    name VARCHAR(100) NOT NULL,
    transport JSONB NOT NULL,
    lifecycle_state VARCHAR(50) NOT NULL DEFAULT 'registered',
    health_status VARCHAR(50) NOT NULL DEFAULT 'unknown',
    health_message TEXT,
    health_checked_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT mcp_servers_lifecycle_state_check CHECK (
        lifecycle_state IN ('registered', 'running', 'stopped')
    ),
    CONSTRAINT mcp_servers_health_status_check CHECK (
        health_status IN ('unknown', 'healthy', 'unhealthy')
    )
);

CREATE UNIQUE INDEX idx_mcp_servers_name
    ON mcp_servers (name);
