-- Revert tenant scoping for hook execution logs.

DROP INDEX idx_hook_executions_tenant_trigger_type_executed_at;
DROP INDEX idx_hook_executions_tenant_trigger_context_id;

CREATE INDEX idx_hook_executions_trigger_type_executed_at
    ON hook_executions (trigger_type, executed_at);

CREATE INDEX idx_hook_executions_trigger_context_id
    ON hook_executions (trigger_context_id);

ALTER TABLE hook_executions
    DROP COLUMN tenant_id;
