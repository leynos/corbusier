-- Backfill nil UUID tenant IDs for conversations and messages before enabling
-- tenant-scoped application queries.
--
-- Business rule:
-- 1. When a conversation is attached to a task, inherit the task's tenant_id.
-- 2. Any remaining conversations with the nil UUID require manual operator
--    resolution before rollout.
--
BEGIN;

LOCK TABLE conversations, messages IN ACCESS EXCLUSIVE MODE;

-- Verify unresolved conversations before backfill:
SELECT id, task_id
FROM conversations
WHERE tenant_id = '00000000-0000-0000-0000-000000000000';

-- Apply deterministic backfill for task-linked conversations.
WITH resolved_conversations AS (
    SELECT conversations.id, tasks.tenant_id
    FROM conversations
    JOIN tasks ON tasks.id = conversations.task_id
    WHERE conversations.tenant_id = '00000000-0000-0000-0000-000000000000'
)
UPDATE conversations
SET tenant_id = resolved_conversations.tenant_id
FROM resolved_conversations
WHERE conversations.id = resolved_conversations.id;

-- Propagate corrected conversation tenant IDs to messages.
UPDATE messages
SET tenant_id = conversations.tenant_id
FROM conversations
WHERE messages.conversation_id = conversations.id
  AND messages.tenant_id IS DISTINCT FROM conversations.tenant_id;

-- Final verification. This must return zero rows before tenant-scoped
-- application queries are enabled.
SELECT id, task_id
FROM conversations
WHERE tenant_id = '00000000-0000-0000-0000-000000000000';

SELECT id, conversation_id
FROM messages
WHERE tenant_id = '00000000-0000-0000-0000-000000000000';

SELECT messages.id,
       messages.conversation_id,
       messages.tenant_id,
       conversations.tenant_id AS conversation_tenant_id
FROM messages
JOIN conversations ON conversations.id = messages.conversation_id
WHERE messages.tenant_id IS DISTINCT FROM conversations.tenant_id;

DO $$
BEGIN
    IF EXISTS (
        SELECT 1
        FROM messages
        JOIN conversations ON conversations.id = messages.conversation_id
        WHERE messages.tenant_id IS DISTINCT FROM conversations.tenant_id
    ) THEN
        RAISE EXCEPTION
            'messages tenant_id mismatch remains after backfill';
    END IF;
END $$;

COMMIT;
