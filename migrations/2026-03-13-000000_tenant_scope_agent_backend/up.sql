-- Tenant-scope agent backend registrations and turn sessions.
--
-- This migration aligns agent backend persistence with RequestContext tenant
-- propagation. It adds tenant_id columns and tenant-aware uniqueness/foreign
-- key constraints while preserving existing rows.

ALTER TABLE backend_registrations
    ADD COLUMN tenant_id UUID NOT NULL
    DEFAULT '00000000-0000-0000-0000-000000000000'::UUID;

ALTER TABLE backend_registrations
    ALTER COLUMN tenant_id DROP DEFAULT;

DROP INDEX IF EXISTS idx_backend_registrations_name;

CREATE UNIQUE INDEX idx_backend_registrations_tenant_name
    ON backend_registrations (tenant_id, name);

CREATE UNIQUE INDEX idx_backend_registrations_id_tenant
    ON backend_registrations (id, tenant_id);

ALTER TABLE agent_turn_sessions
    ADD COLUMN tenant_id UUID;

UPDATE agent_turn_sessions AS sessions
SET tenant_id = backends.tenant_id
FROM backend_registrations AS backends
WHERE sessions.backend_id = backends.id;

ALTER TABLE agent_turn_sessions
    ALTER COLUMN tenant_id SET NOT NULL;

ALTER TABLE agent_turn_sessions
    DROP CONSTRAINT IF EXISTS agent_turn_sessions_backend_id_fkey;

ALTER TABLE agent_turn_sessions
    ADD CONSTRAINT agent_turn_sessions_backend_tenant_fkey
    FOREIGN KEY (backend_id, tenant_id)
    REFERENCES backend_registrations (id, tenant_id)
    ON DELETE CASCADE;

DROP INDEX IF EXISTS idx_agent_turn_sessions_backend_conversation_active;

CREATE UNIQUE INDEX idx_agent_turn_sessions_tenant_backend_conversation_active
    ON agent_turn_sessions (tenant_id, backend_id, conversation_id)
    WHERE status IN ('active', 'reserved');

CREATE INDEX idx_agent_turn_sessions_tenant_backend_conversation
    ON agent_turn_sessions (tenant_id, backend_id, conversation_id);
