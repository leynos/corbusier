-- Revert tenant scoping for agent backend registrations and turn sessions.

DROP INDEX IF EXISTS idx_agent_turn_sessions_tenant_backend_conversation;
DROP INDEX IF EXISTS idx_agent_turn_sessions_tenant_backend_conversation_active;

CREATE UNIQUE INDEX idx_agent_turn_sessions_backend_conversation_active
    ON agent_turn_sessions (backend_id, conversation_id)
    WHERE status = 'active';

ALTER TABLE agent_turn_sessions
    DROP CONSTRAINT IF EXISTS agent_turn_sessions_backend_tenant_fkey;

ALTER TABLE agent_turn_sessions
    ADD CONSTRAINT agent_turn_sessions_backend_id_fkey
    FOREIGN KEY (backend_id)
    REFERENCES backend_registrations (id)
    ON DELETE CASCADE;

ALTER TABLE agent_turn_sessions
    DROP COLUMN tenant_id;

DROP INDEX IF EXISTS idx_backend_registrations_id_tenant;
DROP INDEX IF EXISTS idx_backend_registrations_tenant_name;

CREATE UNIQUE INDEX idx_backend_registrations_name
    ON backend_registrations (name);

ALTER TABLE backend_registrations
    DROP COLUMN tenant_id;
