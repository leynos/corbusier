-- Add turn-session persistence for roadmap item 1.3.2.
-- Stores backend session continuity and expiry lifecycle metadata.

CREATE TABLE agent_turn_sessions (
    id UUID PRIMARY KEY,
    backend_id UUID NOT NULL REFERENCES backend_registrations(id) ON DELETE CASCADE,
    conversation_id UUID NOT NULL,
    runtime_session_id VARCHAR(255) NOT NULL,
    status VARCHAR(20) NOT NULL DEFAULT 'active',
    ttl_seconds BIGINT NOT NULL,
    started_at TIMESTAMPTZ NOT NULL,
    last_used_at TIMESTAMPTZ NOT NULL,
    expires_at TIMESTAMPTZ NOT NULL,
    ended_at TIMESTAMPTZ,
    turn_count BIGINT NOT NULL DEFAULT 0,
    CONSTRAINT agent_turn_sessions_status_check CHECK (
        status IN ('active', 'expired')
    ),
    CONSTRAINT agent_turn_sessions_ttl_positive_check CHECK (
        ttl_seconds > 0
    ),
    CONSTRAINT agent_turn_sessions_turn_count_non_negative_check CHECK (
        turn_count >= 0
    )
);

CREATE UNIQUE INDEX idx_agent_turn_sessions_backend_conversation_active
    ON agent_turn_sessions (backend_id, conversation_id)
    WHERE status = 'active';

CREATE INDEX idx_agent_turn_sessions_expires_at ON agent_turn_sessions (expires_at);
