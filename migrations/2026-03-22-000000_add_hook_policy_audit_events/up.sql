-- Add hook policy audit projection storage for roadmap item 2.3.2.

ALTER TABLE hook_executions
    ADD CONSTRAINT hook_executions_id_tenant_unique
    UNIQUE (id, tenant_id);

CREATE TABLE hook_policy_audit_events (
    id UUID PRIMARY KEY,
    tenant_id UUID NOT NULL,
    hook_execution_id UUID NOT NULL,
    trigger_context_id UUID NOT NULL,
    trigger_type VARCHAR(64) NOT NULL,
    hook_id TEXT NOT NULL,
    action_id TEXT NOT NULL,
    task_id UUID NULL,
    conversation_id UUID NULL,
    decision VARCHAR(32) NOT NULL,
    violation JSONB NULL,
    payload JSONB NOT NULL,
    recorded_at TIMESTAMPTZ NOT NULL,
    CONSTRAINT fk_hook_policy_audit_events_execution
        FOREIGN KEY (hook_execution_id, tenant_id)
        REFERENCES hook_executions (id, tenant_id)
        ON DELETE CASCADE,
    CONSTRAINT chk_hook_policy_audit_events_trigger_type CHECK (
        trigger_type IN (
            'turn_start',
            'turn_end',
            'pre_tool_use',
            'post_tool_use',
            'pre_commit',
            'post_commit',
            'pre_merge',
            'post_merge',
            'pre_pull',
            'post_pull',
            'pre_push',
            'post_push',
            'pre_deploy',
            'post_deploy'
        )
    ),
    CONSTRAINT chk_hook_policy_audit_events_decision CHECK (
        decision IN ('allow', 'deny')
    ),
    CONSTRAINT hook_policy_audit_events_tenant_execution_action_unique
        UNIQUE (tenant_id, hook_execution_id, action_id)
);

CREATE INDEX idx_hook_policy_audit_events_tenant_task_recorded_at
    ON hook_policy_audit_events (tenant_id, task_id, recorded_at)
    WHERE task_id IS NOT NULL;

CREATE INDEX idx_hook_policy_audit_events_tenant_conversation_recorded_at
    ON hook_policy_audit_events (tenant_id, conversation_id, recorded_at)
    WHERE conversation_id IS NOT NULL;

CREATE INDEX idx_hook_policy_audit_events_tenant_trigger_context_recorded_at
    ON hook_policy_audit_events (tenant_id, trigger_context_id, recorded_at);
