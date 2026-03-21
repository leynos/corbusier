-- Add tenant_id to hook execution logs and tenant-scoped lookup indexes.

ALTER TABLE hook_executions
    ADD COLUMN tenant_id UUID NOT NULL
        DEFAULT '00000000-0000-0000-0000-000000000000';

ALTER TABLE hook_executions
    ALTER COLUMN tenant_id DROP DEFAULT;

DROP INDEX idx_hook_executions_trigger_type_executed_at;
DROP INDEX idx_hook_executions_trigger_context_id;

CREATE INDEX idx_hook_executions_tenant_trigger_type_executed_at
    ON hook_executions (tenant_id, trigger_type, executed_at);

CREATE INDEX idx_hook_executions_tenant_trigger_context_id
    ON hook_executions (tenant_id, trigger_context_id);
