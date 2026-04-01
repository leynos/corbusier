-- Revert tenant_id additions to conversations and messages.

DROP INDEX IF EXISTS idx_messages_conversation_tenant;

ALTER TABLE messages DROP COLUMN tenant_id;

DROP INDEX IF EXISTS idx_conversations_id_tenant;

ALTER TABLE conversations DROP COLUMN tenant_id;
