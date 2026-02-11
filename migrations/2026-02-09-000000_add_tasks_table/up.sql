-- Add task lifecycle persistence table for roadmap item 1.2.1
-- Follows corbusier-design.md §2.2.2 and §6.2.1 schema guidance.

CREATE TABLE tasks (
    id UUID PRIMARY KEY,
    origin JSONB NOT NULL,
    -- Reserved for roadmap 1.2.2 branch association.
    branch_ref VARCHAR(255),
    -- Reserved for roadmap 1.2.2 pull-request association.
    pull_request_ref VARCHAR(255),
    state VARCHAR(50) NOT NULL DEFAULT 'draft',
    -- Reserved for roadmap 1.2.3 workspace assignment.
    workspace_id UUID,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT tasks_state_check CHECK (
        state IN ('draft', 'in_progress', 'in_review', 'paused', 'done', 'abandoned')
    )
);

-- Enforce one task per external issue reference for issue-origin tasks.
CREATE UNIQUE INDEX idx_tasks_issue_origin_unique ON tasks (
    (origin->'issue_ref'->>'provider'),
    (origin->'issue_ref'->>'repository'),
    ((origin->'issue_ref'->>'issue_number')::BIGINT)
) WHERE origin->>'type' = 'issue';
