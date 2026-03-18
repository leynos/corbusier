-- Add UNIQUE constraint to prevent duplicate hook executions and enable idempotent pending execution storage.
-- This constraint ensures that storing a pending execution record is safe to retry.

ALTER TABLE hook_executions
    ADD CONSTRAINT hook_executions_tenant_context_hook_unique
    UNIQUE (tenant_id, trigger_context_id, hook_id);

-- Update status CHECK constraint to include 'pending' status.
ALTER TABLE hook_executions
    DROP CONSTRAINT chk_hook_executions_status;

ALTER TABLE hook_executions
    ADD CONSTRAINT chk_hook_executions_status CHECK (
        status IN ('pending', 'succeeded', 'failed', 'partial_failure')
    );
