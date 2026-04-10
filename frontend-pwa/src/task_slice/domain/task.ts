/**
 * Shared task-slice domain contract used by adapters, hooks, and UI modules.
 *
 * The types in this module define task state, issue and review references, and
 * request/view-model contracts that should remain stable across slice layers.
 */
export type TaskState =
  | 'draft'
  | 'in_progress'
  | 'in_review'
  | 'paused'
  | 'done'
  | 'abandoned';

export type IssueProvider = 'github' | 'gitlab';

export interface IssueRef {
  provider: IssueProvider;
  repository: string;
  issue_number: number;
}

export interface IssueSnapshot {
  title: string;
  description?: string;
  labels: string[];
  assignees: string[];
  milestone?: string;
}

export interface TaskOrigin {
  type: 'issue';
  issue_ref: IssueRef;
  metadata: IssueSnapshot;
}

export interface BranchRef {
  provider: IssueProvider;
  repository: string;
  branch_name: string;
}

export interface PullRequestRef {
  provider: IssueProvider;
  repository: string;
  pull_request_number: number;
}

export interface Task {
  id: string;
  origin: TaskOrigin;
  branch_ref?: BranchRef;
  pull_request_ref?: PullRequestRef;
  state: TaskState;
  created_at: string;
  updated_at: string;
}

export interface CreateTaskRequest {
  provider: IssueProvider;
  repository: string;
  issue_number: number;
  title: string;
  description?: string;
  labels?: string[];
  assignees?: string[];
  milestone?: string;
}

export interface TaskStateViewModel {
  label: string;
  tone: 'steady' | 'warning';
}
