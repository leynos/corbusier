BEGIN;

-- Tenant-scope conversations table
ALTER TABLE conversations
    ADD COLUMN tenant_id UUID NOT NULL
        DEFAULT '00000000-0000-0000-0000-000000000001';

ALTER TABLE conversations
    ALTER COLUMN tenant_id DROP DEFAULT;

-- Create unique index on (id, tenant_id) for tenant-consistent lookups
CREATE UNIQUE INDEX idx_conversations_id_tenant
    ON conversations (id, tenant_id);

-- Tenant-scope messages table
ALTER TABLE messages
    ADD COLUMN tenant_id UUID NOT NULL
        DEFAULT '00000000-0000-0000-0000-000000000001';

ALTER TABLE messages
    ALTER COLUMN tenant_id DROP DEFAULT;

-- Create unique index on (id, tenant_id) for tenant-consistent lookups
CREATE UNIQUE INDEX idx_messages_id_tenant
    ON messages (id, tenant_id);

-- Create index for tenant-scoped message lookups by conversation
CREATE INDEX idx_messages_conversation_tenant
    ON messages (conversation_id, tenant_id);

COMMIT;
