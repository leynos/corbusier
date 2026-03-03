-- Add hook executions table for roadmap item 2.3.1
-- Follows corbusier-design.md §6.2.1.

CREATE TABLE hook_executions (
    id UUID PRIMARY KEY,
    trigger_context_id UUID NOT NULL,
    hook_id VARCHAR(255) NOT NULL,
    trigger_type VARCHAR(64) NOT NULL,
    predicate_data JSONB NOT NULL,
    action_results JSONB NOT NULL,
    status VARCHAR(32) NOT NULL,
    executed_at TIMESTAMPTZ NOT NULL
);

CREATE INDEX idx_hook_executions_trigger_type_executed_at
    ON hook_executions (trigger_type, executed_at);

CREATE INDEX idx_hook_executions_trigger_context_id
    ON hook_executions (trigger_context_id);
