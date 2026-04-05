-- Enforce tenant-aware integrity for conversations and messages.

DO $$
BEGIN
    IF NOT EXISTS (
        SELECT 1
        FROM pg_constraint
        WHERE conname = 'conversations_id_tenant_unique'
    ) THEN
        ALTER TABLE conversations
            ADD CONSTRAINT conversations_id_tenant_unique
            UNIQUE (id, tenant_id);
    END IF;
END $$;

DO $$
BEGIN
    IF NOT EXISTS (
        SELECT 1
        FROM pg_constraint
        WHERE conname = 'messages_id_tenant_unique'
    ) THEN
        ALTER TABLE messages
            ADD CONSTRAINT messages_id_tenant_unique
            UNIQUE (id, tenant_id);
    END IF;
END $$;

ALTER TABLE messages
    DROP CONSTRAINT IF EXISTS messages_conversation_id_fkey,
    DROP CONSTRAINT IF EXISTS messages_conversation_fk;

DO $$
BEGIN
    IF NOT EXISTS (
        SELECT 1
        FROM pg_constraint
        WHERE conrelid = 'messages'::regclass
          AND conname = 'messages_conversation_tenant_fkey'
    ) THEN
        ALTER TABLE messages
            ADD CONSTRAINT messages_conversation_tenant_fkey
            FOREIGN KEY (conversation_id, tenant_id)
            REFERENCES conversations (id, tenant_id)
            ON DELETE CASCADE;
    END IF;
END $$;
