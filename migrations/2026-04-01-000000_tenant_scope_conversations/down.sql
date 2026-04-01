BEGIN;

DROP INDEX IF EXISTS idx_messages_conversation_tenant;
DROP INDEX IF EXISTS idx_messages_id_tenant;
DROP INDEX IF EXISTS idx_conversations_id_tenant;

ALTER TABLE messages
    DROP COLUMN tenant_id;

ALTER TABLE conversations
    DROP COLUMN tenant_id;

COMMIT;
