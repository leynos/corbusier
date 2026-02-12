-- Non-unique index for branch reference lookups (many-to-many: multiple
-- tasks may share a branch). Follows corbusier-design.md §2.2.2.
CREATE INDEX idx_tasks_branch_ref ON tasks (branch_ref)
    WHERE branch_ref IS NOT NULL;

-- Non-unique index for pull request reference lookups.
CREATE INDEX idx_tasks_pull_request_ref ON tasks (pull_request_ref)
    WHERE pull_request_ref IS NOT NULL;
