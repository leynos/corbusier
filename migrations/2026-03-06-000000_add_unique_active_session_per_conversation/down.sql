-- Revert to the original non-unique partial index.

DROP INDEX IF EXISTS idx_agent_sessions_one_active_per_conversation;

CREATE INDEX idx_agent_sessions_conversation_state
    ON agent_sessions (conversation_id, state)
    WHERE state = 'active';
