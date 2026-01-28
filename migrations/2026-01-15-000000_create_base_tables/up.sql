-- Create base tables for message persistence
-- Follows corbusier-design.md section 6.2.3

-- Requires superuser or managed-service roles (e.g. azure_pg_admin,
-- cloudsqlsuperuser, or the appropriate RDS role). IF NOT EXISTS avoids errors
-- when the extension is pre-provisioned; ensure deploys use a privileged
-- account or have a DBA enable pgcrypto ahead of time.
CREATE EXTENSION IF NOT EXISTS pgcrypto;

-- Conversations table stores conversation metadata
CREATE TABLE conversations (
    id UUID PRIMARY KEY,
    task_id UUID,
    context JSONB NOT NULL DEFAULT '{}',
    state VARCHAR(50) NOT NULL DEFAULT 'active',
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Messages table stores conversation messages with append-only semantics
CREATE TABLE messages (
    id UUID PRIMARY KEY,
    conversation_id UUID NOT NULL REFERENCES conversations(id),
    role VARCHAR(20) NOT NULL,
    content JSONB NOT NULL,
    metadata JSONB NOT NULL DEFAULT '{}',
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    sequence_number BIGINT NOT NULL,
    -- Enforce per-conversation sequence uniqueness at the database level
    CONSTRAINT messages_conversation_sequence_unique UNIQUE (conversation_id, sequence_number)
);

-- Domain events table for event sourcing and audit trails
CREATE TABLE domain_events (
    id UUID PRIMARY KEY,
    aggregate_id UUID NOT NULL,
    aggregate_type VARCHAR(100) NOT NULL,
    event_type VARCHAR(100) NOT NULL,
    event_data JSONB NOT NULL,
    event_version INT NOT NULL DEFAULT 1,
    occurred_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    correlation_id UUID,
    causation_id UUID,
    user_id UUID,
    session_id UUID
);

-- Audit logs table for compliance tracking
CREATE TABLE audit_logs (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    table_name VARCHAR(100) NOT NULL,
    operation VARCHAR(10) NOT NULL,
    row_id UUID,
    old_values JSONB,
    new_values JSONB,
    user_id UUID,
    session_id UUID,
    correlation_id UUID,
    causation_id UUID,
    application_name VARCHAR(100),
    occurred_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Indexes for common query patterns
CREATE INDEX idx_messages_conversation_id ON messages(conversation_id);
-- Note: The UNIQUE constraint on (conversation_id, sequence_number) automatically
-- creates an index, so no explicit idx_messages_conversation_sequence is needed.
CREATE INDEX idx_domain_events_aggregate ON domain_events(aggregate_id, aggregate_type);
-- Partial index excludes NULL values for space efficiency
CREATE INDEX idx_domain_events_correlation ON domain_events(correlation_id)
    WHERE correlation_id IS NOT NULL;
CREATE INDEX idx_audit_logs_table_row ON audit_logs(table_name, row_id);
