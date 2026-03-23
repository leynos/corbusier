BEGIN;

CREATE TABLE tenants (
    id UUID PRIMARY KEY,
    slug VARCHAR(63) NOT NULL,
    name VARCHAR(255) NOT NULL,
    status VARCHAR(20) NOT NULL DEFAULT 'active',
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT tenants_slug_unique UNIQUE (slug),
    CONSTRAINT tenants_status_check CHECK (
        status IN ('active', 'suspended')
    )
);

INSERT INTO tenants (id, slug, name, status)
VALUES ('00000000-0000-0000-0000-000000000001', 'default', 'Default Tenant', 'active');

-- Add tenant_id columns to tables (backend_registrations already has it from migration 2026-03-13)
ALTER TABLE tasks
    ADD COLUMN tenant_id UUID NOT NULL
        REFERENCES tenants(id)
        DEFAULT '00000000-0000-0000-0000-000000000001';
ALTER TABLE conversations
    ADD COLUMN tenant_id UUID NOT NULL
        REFERENCES tenants(id)
        DEFAULT '00000000-0000-0000-0000-000000000001';
ALTER TABLE messages
    ADD COLUMN tenant_id UUID NOT NULL
        REFERENCES tenants(id)
        DEFAULT '00000000-0000-0000-0000-000000000001';
ALTER TABLE agent_sessions
    ADD COLUMN tenant_id UUID NOT NULL
        REFERENCES tenants(id)
        DEFAULT '00000000-0000-0000-0000-000000000001';
ALTER TABLE handoffs
    ADD COLUMN tenant_id UUID NOT NULL
        REFERENCES tenants(id)
        DEFAULT '00000000-0000-0000-0000-000000000001';
ALTER TABLE context_snapshots
    ADD COLUMN tenant_id UUID NOT NULL
        REFERENCES tenants(id)
        DEFAULT '00000000-0000-0000-0000-000000000001';

-- Add FK constraint from backend_registrations.tenant_id to tenants table
-- (the column itself was added by migration 2026-03-13)
ALTER TABLE backend_registrations
    ADD CONSTRAINT backend_registrations_tenant_fk
        FOREIGN KEY (tenant_id)
        REFERENCES tenants(id)
        ON DELETE CASCADE;

-- Drop defaults after initial migration
ALTER TABLE tasks
    ALTER COLUMN tenant_id DROP DEFAULT;
ALTER TABLE conversations
    ALTER COLUMN tenant_id DROP DEFAULT;
ALTER TABLE messages
    ALTER COLUMN tenant_id DROP DEFAULT;
ALTER TABLE agent_sessions
    ALTER COLUMN tenant_id DROP DEFAULT;
ALTER TABLE handoffs
    ALTER COLUMN tenant_id DROP DEFAULT;
ALTER TABLE context_snapshots
    ALTER COLUMN tenant_id DROP DEFAULT;

-- Drop old non-tenant-scoped indexes
DROP INDEX IF EXISTS idx_tasks_issue_origin_unique;
DROP INDEX IF EXISTS idx_tasks_branch_ref;
DROP INDEX IF EXISTS idx_tasks_pull_request_ref;
-- idx_backend_registrations_name already dropped by migration 2026-03-13
DROP INDEX IF EXISTS idx_agent_sessions_one_active_per_conversation;
DROP INDEX IF EXISTS idx_agent_sessions_conversation_id;
DROP INDEX IF EXISTS idx_agent_sessions_conversation_state;

-- Drop old foreign key constraints before adding composite ones
ALTER TABLE messages
    DROP CONSTRAINT IF EXISTS messages_conversation_id_fkey;
ALTER TABLE agent_sessions
    DROP CONSTRAINT IF EXISTS agent_sessions_conversation_id_fkey;
ALTER TABLE handoffs
    DROP CONSTRAINT IF EXISTS handoffs_source_session_id_fkey;
ALTER TABLE handoffs
    DROP CONSTRAINT IF EXISTS handoffs_conversation_id_fkey;
ALTER TABLE handoffs
    DROP CONSTRAINT IF EXISTS handoffs_target_session_id_fkey;
ALTER TABLE context_snapshots
    DROP CONSTRAINT IF EXISTS context_snapshots_conversation_id_fkey;
ALTER TABLE context_snapshots
    DROP CONSTRAINT IF EXISTS context_snapshots_session_id_fkey;
ALTER TABLE agent_sessions
    DROP CONSTRAINT IF EXISTS agent_sessions_initiated_by_handoff_fk;
ALTER TABLE agent_sessions
    DROP CONSTRAINT IF EXISTS agent_sessions_terminated_by_handoff_fk;

-- Create unique indexes for composite FK targets
CREATE UNIQUE INDEX idx_tasks_id_tenant
    ON tasks (id, tenant_id);
-- idx_backend_registrations_id_tenant already created by migration 2026-03-13
CREATE UNIQUE INDEX idx_conversations_id_tenant
    ON conversations (id, tenant_id);
CREATE UNIQUE INDEX idx_agent_sessions_id_tenant
    ON agent_sessions (id, tenant_id);
CREATE UNIQUE INDEX idx_handoffs_id_tenant
    ON handoffs (id, tenant_id);

-- Create tenant-scoped indexes
CREATE UNIQUE INDEX idx_tasks_issue_origin_unique ON tasks (
    tenant_id,
    (origin->'issue_ref'->>'provider'),
    (origin->'issue_ref'->>'repository'),
    ((origin->'issue_ref'->>'issue_number')::BIGINT)
) WHERE origin->>'type' = 'issue';

CREATE INDEX idx_tasks_branch_ref ON tasks (tenant_id, branch_ref)
    WHERE branch_ref IS NOT NULL;

CREATE INDEX idx_tasks_pull_request_ref ON tasks (tenant_id, pull_request_ref)
    WHERE pull_request_ref IS NOT NULL;

-- idx_backend_registrations_tenant_name already created by migration 2026-03-13

CREATE INDEX idx_backend_registrations_tenant_status
    ON backend_registrations (tenant_id, status);

CREATE INDEX idx_conversations_tenant_task_id
    ON conversations (tenant_id, task_id)
    WHERE task_id IS NOT NULL;

CREATE INDEX idx_messages_tenant_conversation_id
    ON messages (tenant_id, conversation_id);

CREATE INDEX idx_messages_tenant_conversation_sequence
    ON messages (tenant_id, conversation_id, sequence_number);

CREATE INDEX idx_agent_sessions_conversation_id
    ON agent_sessions (tenant_id, conversation_id);

CREATE UNIQUE INDEX idx_agent_sessions_one_active_per_conversation
    ON agent_sessions (tenant_id, conversation_id)
    WHERE state = 'active';

-- Create composite foreign key constraints
ALTER TABLE conversations
    ADD CONSTRAINT conversations_task_fk
        FOREIGN KEY (task_id, tenant_id)
        REFERENCES tasks (id, tenant_id)
        ON DELETE SET NULL;

ALTER TABLE messages
    ADD CONSTRAINT messages_conversation_fk
        FOREIGN KEY (conversation_id, tenant_id)
        REFERENCES conversations (id, tenant_id)
        ON DELETE CASCADE;

ALTER TABLE agent_sessions
    ADD CONSTRAINT agent_sessions_conversation_fk
        FOREIGN KEY (conversation_id, tenant_id)
        REFERENCES conversations (id, tenant_id)
        ON DELETE CASCADE;

ALTER TABLE handoffs
    ADD CONSTRAINT handoffs_source_session_fk
        FOREIGN KEY (source_session_id, tenant_id)
        REFERENCES agent_sessions (id, tenant_id)
        ON DELETE CASCADE;

ALTER TABLE handoffs
    ADD CONSTRAINT handoffs_conversation_fk
        FOREIGN KEY (conversation_id, tenant_id)
        REFERENCES conversations (id, tenant_id)
        ON DELETE CASCADE;

ALTER TABLE handoffs
    ADD CONSTRAINT handoffs_target_session_fk
        FOREIGN KEY (target_session_id, tenant_id)
        REFERENCES agent_sessions (id, tenant_id)
        ON DELETE SET NULL;

ALTER TABLE context_snapshots
    ADD CONSTRAINT context_snapshots_conversation_fk
        FOREIGN KEY (conversation_id, tenant_id)
        REFERENCES conversations (id, tenant_id)
        ON DELETE CASCADE;

ALTER TABLE context_snapshots
    ADD CONSTRAINT context_snapshots_session_fk
        FOREIGN KEY (session_id, tenant_id)
        REFERENCES agent_sessions (id, tenant_id)
        ON DELETE CASCADE;

ALTER TABLE agent_sessions
    ADD CONSTRAINT agent_sessions_initiated_by_handoff_fk
        FOREIGN KEY (initiated_by_handoff, tenant_id)
        REFERENCES handoffs (id, tenant_id)
        ON DELETE SET NULL;

ALTER TABLE agent_sessions
    ADD CONSTRAINT agent_sessions_terminated_by_handoff_fk
        FOREIGN KEY (terminated_by_handoff, tenant_id)
        REFERENCES handoffs (id, tenant_id)
        ON DELETE SET NULL;

COMMIT;
