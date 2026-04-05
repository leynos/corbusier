-- Revert tenant-aware integrity enforcement for conversations and messages.

ALTER TABLE messages
    DROP CONSTRAINT messages_conversation_tenant_fkey;

ALTER TABLE messages
    ADD CONSTRAINT messages_conversation_id_fkey
    FOREIGN KEY (conversation_id)
    REFERENCES conversations (id);

ALTER TABLE messages
    DROP CONSTRAINT messages_id_tenant_unique;

ALTER TABLE conversations
    DROP CONSTRAINT conversations_id_tenant_unique;
