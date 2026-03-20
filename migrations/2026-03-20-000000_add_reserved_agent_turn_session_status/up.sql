-- Allow reserved turn-session rows during atomic slot claims.

ALTER TABLE agent_turn_sessions
    DROP CONSTRAINT IF EXISTS agent_turn_sessions_status_check;

ALTER TABLE agent_turn_sessions
    ADD CONSTRAINT agent_turn_sessions_status_check CHECK (
        status IN ('active', 'reserved', 'expired')
    );

DROP INDEX IF EXISTS idx_agent_turn_sessions_tenant_backend_conversation_active;

CREATE UNIQUE INDEX idx_agent_turn_sessions_tenant_backend_conversation_active
    ON agent_turn_sessions (tenant_id, backend_id, conversation_id)
    WHERE status IN ('active', 'reserved');
