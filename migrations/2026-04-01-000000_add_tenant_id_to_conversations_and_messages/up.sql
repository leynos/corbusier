-- Add tenant_id to conversations and messages for tenant isolation.

-- 1. conversations
ALTER TABLE conversations
    ADD COLUMN IF NOT EXISTS tenant_id UUID NOT NULL
        DEFAULT '00000000-0000-0000-0000-000000000000';

ALTER TABLE conversations
    ALTER COLUMN tenant_id DROP DEFAULT;

CREATE INDEX IF NOT EXISTS idx_conversations_id_tenant
    ON conversations (id, tenant_id);

-- 2. messages
ALTER TABLE messages
    ADD COLUMN IF NOT EXISTS tenant_id UUID NOT NULL
        DEFAULT '00000000-0000-0000-0000-000000000000';

ALTER TABLE messages
    ALTER COLUMN tenant_id DROP DEFAULT;

CREATE INDEX IF NOT EXISTS idx_messages_conversation_tenant
    ON messages (conversation_id, tenant_id);
