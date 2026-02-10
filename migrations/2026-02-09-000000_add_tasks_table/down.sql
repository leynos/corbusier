-- Revert task lifecycle persistence table.

DROP INDEX IF EXISTS idx_tasks_issue_origin_unique;
DROP TABLE IF EXISTS tasks;
