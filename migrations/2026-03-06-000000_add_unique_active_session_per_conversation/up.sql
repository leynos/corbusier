-- Enforce at most one active agent session per conversation at the database
-- level via a partial unique index.  This replaces the non-unique partial
-- index created in the original agent_sessions migration.

DROP INDEX IF EXISTS idx_agent_sessions_conversation_state;

CREATE UNIQUE INDEX idx_agent_sessions_one_active_per_conversation
    ON agent_sessions (conversation_id)
    WHERE state = 'active';
