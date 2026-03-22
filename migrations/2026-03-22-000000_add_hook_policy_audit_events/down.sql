DROP TABLE IF EXISTS hook_policy_audit_events;

ALTER TABLE hook_executions
    DROP CONSTRAINT IF EXISTS hook_executions_id_tenant_unique;
