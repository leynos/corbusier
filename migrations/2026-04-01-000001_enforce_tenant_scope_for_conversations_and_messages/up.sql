-- Enforce tenant-aware integrity for conversations and messages.

ALTER TABLE conversations
    ADD CONSTRAINT conversations_id_tenant_unique
    UNIQUE (id, tenant_id);

ALTER TABLE messages
    ADD CONSTRAINT messages_id_tenant_unique
    UNIQUE (id, tenant_id);

ALTER TABLE messages
    DROP CONSTRAINT messages_conversation_id_fkey;

ALTER TABLE messages
    ADD CONSTRAINT messages_conversation_tenant_fkey
    FOREIGN KEY (conversation_id, tenant_id)
    REFERENCES conversations (id, tenant_id)
    ON DELETE CASCADE;
