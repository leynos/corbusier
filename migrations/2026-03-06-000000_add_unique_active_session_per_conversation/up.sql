-- Enforce at most one active agent session per conversation at the database
-- level via a partial unique index.  This replaces the non-unique partial
-- index created in the original agent_sessions migration.

-- Preflight: abort if any conversation already has more than one active
-- session, since the unique index creation would fail and the root cause
-- would be unclear from a bare constraint error.
DO $$
DECLARE
    _violations RECORD;
    _msg       TEXT;
BEGIN
    SELECT conversation_id, COUNT(*) AS n
      INTO _violations
      FROM agent_sessions
     WHERE state = 'active'
     GROUP BY conversation_id
    HAVING COUNT(*) > 1
     LIMIT 1;

    IF FOUND THEN
        _msg := format(
            'Cannot create unique active-session index: conversation %s has %s active sessions. '
            'Remediate duplicates before re-running this migration.',
            _violations.conversation_id, _violations.n
        );
        RAISE EXCEPTION '%', _msg;
    END IF;
END
$$;

DROP INDEX IF EXISTS idx_agent_sessions_conversation_state;

CREATE UNIQUE INDEX idx_agent_sessions_one_active_per_conversation
    ON agent_sessions (conversation_id)
    WHERE state = 'active';
