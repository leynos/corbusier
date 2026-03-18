-- Revert status CHECK constraint to exclude 'pending'.
ALTER TABLE hook_executions
    DROP CONSTRAINT chk_hook_executions_status;

ALTER TABLE hook_executions
    ADD CONSTRAINT chk_hook_executions_status CHECK (
        status IN ('succeeded', 'failed', 'partial_failure')
    );

-- Drop the UNIQUE constraint.
ALTER TABLE hook_executions
    DROP CONSTRAINT hook_executions_tenant_context_hook_unique;
