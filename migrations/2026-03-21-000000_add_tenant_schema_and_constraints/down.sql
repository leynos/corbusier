BEGIN;

-- Preflight: abort rollback if multi-tenant data would violate global UNIQUE indexes.
-- The up migration scopes uniqueness by tenant, so rolling back to global uniqueness
-- will fail if multiple tenants share the same issue reference or backend name.
DO $$
DECLARE
    _task_duplicates INTEGER;
    _backend_duplicates INTEGER;
BEGIN
    -- Check for duplicate issue references across tenants
    SELECT COUNT(*)
      INTO _task_duplicates
      FROM (
        SELECT
            origin->'issue_ref'->>'provider' AS provider,
            origin->'issue_ref'->>'repository' AS repository,
            (origin->'issue_ref'->>'issue_number')::BIGINT AS issue_number,
            COUNT(DISTINCT tenant_id) AS tenant_count
        FROM tasks
        WHERE origin->>'type' = 'issue'
        GROUP BY provider, repository, issue_number
        HAVING COUNT(DISTINCT tenant_id) > 1
      ) duplicates;

    IF _task_duplicates > 0 THEN
        RAISE EXCEPTION 'Cannot rollback: % issue reference(s) exist in multiple tenants. '
            'Global UNIQUE constraint idx_tasks_issue_origin_unique would fail. '
            'Remediate cross-tenant duplicates or accept irreversibility.',
            _task_duplicates;
    END IF;

    -- Check for duplicate backend names across tenants
    SELECT COUNT(*)
      INTO _backend_duplicates
      FROM (
        SELECT name, COUNT(DISTINCT tenant_id) AS tenant_count
        FROM backend_registrations
        GROUP BY name
        HAVING COUNT(DISTINCT tenant_id) > 1
      ) duplicates;

    IF _backend_duplicates > 0 THEN
        RAISE EXCEPTION 'Cannot rollback: % backend name(s) exist in multiple tenants. '
            'Global UNIQUE constraint idx_backend_registrations_name would fail. '
            'Remediate cross-tenant duplicates or accept irreversibility.',
            _backend_duplicates;
    END IF;
END
$$;

ALTER TABLE agent_sessions
    DROP CONSTRAINT IF EXISTS agent_sessions_terminated_by_handoff_fk;
ALTER TABLE agent_sessions
    DROP CONSTRAINT IF EXISTS agent_sessions_initiated_by_handoff_fk;
ALTER TABLE context_snapshots
    DROP CONSTRAINT IF EXISTS context_snapshots_session_fk;
ALTER TABLE context_snapshots
    DROP CONSTRAINT IF EXISTS context_snapshots_conversation_fk;
ALTER TABLE handoffs
    DROP CONSTRAINT IF EXISTS handoffs_target_session_fk;
ALTER TABLE handoffs
    DROP CONSTRAINT IF EXISTS handoffs_conversation_fk;
ALTER TABLE handoffs
    DROP CONSTRAINT IF EXISTS handoffs_source_session_fk;
ALTER TABLE agent_sessions
    DROP CONSTRAINT IF EXISTS agent_sessions_conversation_fk;
ALTER TABLE messages
    DROP CONSTRAINT IF EXISTS messages_conversation_fk;
ALTER TABLE conversations
    DROP CONSTRAINT IF EXISTS conversations_task_fk;

DROP INDEX IF EXISTS idx_context_snapshots_tenant_conversation_captured_at;
DROP INDEX IF EXISTS idx_context_snapshots_tenant_session_captured_at;
DROP INDEX IF EXISTS idx_agent_sessions_one_active_per_conversation;
DROP INDEX IF EXISTS idx_agent_sessions_conversation_id;
DROP INDEX IF EXISTS idx_messages_tenant_conversation_sequence;
DROP INDEX IF EXISTS idx_messages_tenant_conversation_id;
DROP INDEX IF EXISTS idx_conversations_tenant_task_id;
DROP INDEX IF EXISTS idx_backend_registrations_tenant_status;
DROP INDEX IF EXISTS idx_backend_registrations_name;
DROP INDEX IF EXISTS idx_tasks_pull_request_ref;
DROP INDEX IF EXISTS idx_tasks_branch_ref;
DROP INDEX IF EXISTS idx_tasks_issue_origin_unique;
DROP INDEX IF EXISTS idx_handoffs_id_tenant;
DROP INDEX IF EXISTS idx_agent_sessions_id_tenant;
DROP INDEX IF EXISTS idx_conversations_id_tenant;
DROP INDEX IF EXISTS idx_backend_registrations_id_tenant;
DROP INDEX IF EXISTS idx_tasks_id_tenant;

ALTER TABLE messages
    ADD CONSTRAINT messages_conversation_id_fkey
        FOREIGN KEY (conversation_id)
        REFERENCES conversations (id);

ALTER TABLE agent_sessions
    ADD CONSTRAINT agent_sessions_conversation_id_fkey
        FOREIGN KEY (conversation_id)
        REFERENCES conversations (id)
        ON DELETE CASCADE;

ALTER TABLE handoffs
    ADD CONSTRAINT handoffs_source_session_id_fkey
        FOREIGN KEY (source_session_id)
        REFERENCES agent_sessions (id)
        ON DELETE CASCADE;

ALTER TABLE handoffs
    ADD CONSTRAINT handoffs_conversation_id_fkey
        FOREIGN KEY (conversation_id)
        REFERENCES conversations (id)
        ON DELETE CASCADE;

ALTER TABLE handoffs
    ADD CONSTRAINT handoffs_target_session_id_fkey
        FOREIGN KEY (target_session_id)
        REFERENCES agent_sessions (id)
        ON DELETE SET NULL;

ALTER TABLE context_snapshots
    ADD CONSTRAINT context_snapshots_conversation_id_fkey
        FOREIGN KEY (conversation_id)
        REFERENCES conversations (id)
        ON DELETE CASCADE;

ALTER TABLE context_snapshots
    ADD CONSTRAINT context_snapshots_session_id_fkey
        FOREIGN KEY (session_id)
        REFERENCES agent_sessions (id)
        ON DELETE CASCADE;

ALTER TABLE agent_sessions
    ADD CONSTRAINT agent_sessions_initiated_by_handoff_fk
        FOREIGN KEY (initiated_by_handoff)
        REFERENCES handoffs (id)
        ON DELETE SET NULL;

ALTER TABLE agent_sessions
    ADD CONSTRAINT agent_sessions_terminated_by_handoff_fk
        FOREIGN KEY (terminated_by_handoff)
        REFERENCES handoffs (id)
        ON DELETE SET NULL;

CREATE UNIQUE INDEX idx_tasks_issue_origin_unique ON tasks (
    (origin->'issue_ref'->>'provider'),
    (origin->'issue_ref'->>'repository'),
    ((origin->'issue_ref'->>'issue_number')::BIGINT)
) WHERE origin->>'type' = 'issue';

CREATE INDEX idx_tasks_branch_ref ON tasks (branch_ref)
    WHERE branch_ref IS NOT NULL;

CREATE INDEX idx_tasks_pull_request_ref ON tasks (pull_request_ref)
    WHERE pull_request_ref IS NOT NULL;

CREATE UNIQUE INDEX idx_backend_registrations_name
    ON backend_registrations (name);

CREATE INDEX idx_agent_sessions_conversation_id
    ON agent_sessions (conversation_id);

CREATE UNIQUE INDEX idx_agent_sessions_one_active_per_conversation
    ON agent_sessions (conversation_id)
    WHERE state = 'active';

ALTER TABLE context_snapshots
    DROP COLUMN tenant_id;
ALTER TABLE handoffs
    DROP COLUMN tenant_id;
ALTER TABLE agent_sessions
    DROP COLUMN tenant_id;
ALTER TABLE messages
    DROP COLUMN tenant_id;
ALTER TABLE conversations
    DROP COLUMN tenant_id;
ALTER TABLE backend_registrations
    DROP COLUMN tenant_id;
ALTER TABLE tasks
    DROP COLUMN tenant_id;

DROP TABLE tenants;

COMMIT;
