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

/** Source control provider that hosts the originating issue. */
export type IssueProvider = 'github' | 'gitlab';

/** Locates the issue a task originated from within its host provider. */
export interface IssueRef {
  /** Host provider for {@link IssueRef.repository}. */
  provider: IssueProvider;
  /** Owner-qualified repository, for example `owner/repository`. */
  repository: string;
  /** Provider-native issue number, unique within the repository. */
  issue_number: number;
}

/** Point-in-time copy of the originating issue's user-facing fields. */
export interface IssueSnapshot {
  /** Issue title as of the snapshot. */
  title: string;
  /** Issue body as of the snapshot, if the issue had one. */
  description?: string;
  /** Labels applied to the issue as of the snapshot. */
  labels: string[];
  /** Assignees on the issue as of the snapshot. */
  assignees: string[];
  /** Milestone attached to the issue as of the snapshot, if any. */
  milestone?: string;
}

/** Records how a task came into existence; currently issue-only. */
export interface TaskOrigin {
  /** Discriminant; only issue-sourced tasks exist today. */
  type: 'issue';
  /** Issue the task was created from. */
  issue_ref: IssueRef;
  /** Snapshot taken at task creation; may drift from the live issue. */
  metadata: IssueSnapshot;
}

/** Identifies the working branch created for a task, once one exists. */
export interface BranchRef {
  /** Host provider for {@link BranchRef.repository}. */
  provider: IssueProvider;
  /** Owner-qualified repository, for example `owner/repository`. */
  repository: string;
  /** Name of the working branch on the host provider. */
  branch_name: string;
}

/** Identifies the pull request opened for a task, once one exists. */
export interface PullRequestRef {
  /** Host provider for {@link PullRequestRef.repository}. */
  provider: IssueProvider;
  /** Owner-qualified repository, for example `owner/repository`. */
  repository: string;
  /** Provider-native pull request number, unique within the repository. */
  pull_request_number: number;
}

/** A unit of intake work tracked through the task lifecycle. */
export interface Task {
  /** Corbusier-assigned task identifier. */
  id: string;
  /** Where the task originated; currently always an issue. */
  origin: TaskOrigin;
  /** Working branch once one has been created for this task. */
  branch_ref?: BranchRef;
  /** Pull request once one has been opened for this task. */
  pull_request_ref?: PullRequestRef;
  /** Current lifecycle state. */
  state: TaskState;
  /** ISO 8601 timestamp of task creation. */
  created_at: string;
  /** ISO 8601 timestamp of the most recent task update. */
  updated_at: string;
}

/** Payload accepted by the gateway to create a task from an issue. */
export interface CreateTaskRequest {
  /** Host provider for {@link CreateTaskRequest.repository}. */
  provider: IssueProvider;
  /** Owner-qualified repository, for example `owner/repository`. */
  repository: string;
  /** Provider-native issue number to create the task from. */
  issue_number: number;
  /** Task title, seeded from the issue title. */
  title: string;
  /** Task description, seeded from the issue body. */
  description?: string;
  /** Labels to carry over from the issue. */
  labels?: string[];
  /** Assignees to carry over from the issue. */
  assignees?: string[];
  /** Milestone to carry over from the issue, if any. */
  milestone?: string;
}

/** UI-ready rendering of a task state: display text plus visual tone. */
export interface TaskStateViewModel {
  /** Localized display text for the state. */
  label: string;
  /** Visual emphasis; `warning` flags paused or abandoned states. */
  tone: 'steady' | 'warning';
}
