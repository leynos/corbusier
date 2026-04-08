import type {
  BranchRef,
  PullRequestRef,
  TaskOrigin,
  TaskState,
  TaskStateViewModel,
} from './task';

export function formatIssueOrigin(origin: TaskOrigin) {
  const issueRef = origin.issue_ref;
  return `${issueRef.provider}/${issueRef.repository}/#${issueRef.issue_number}`;
}

export function formatTaskState(state: TaskState): TaskStateViewModel {
  switch (state) {
    case 'draft':
      return { label: 'Draft', tone: 'steady' };
    case 'in_progress':
      return { label: 'In progress', tone: 'steady' };
    case 'in_review':
      return { label: 'In review', tone: 'steady' };
    case 'paused':
      return { label: 'Paused', tone: 'warning' };
    case 'done':
      return { label: 'Done', tone: 'steady' };
    case 'abandoned':
      return { label: 'Abandoned', tone: 'warning' };
  }
}

export function formatTimestamp(value: string, locale = 'en-GB') {
  return new Intl.DateTimeFormat(locale, {
    dateStyle: 'medium',
    timeStyle: 'short',
    timeZone: 'UTC',
  }).format(new Date(value));
}

export function formatBranchRef(branchRef?: BranchRef) {
  return branchRef
    ? `${branchRef.provider}:${branchRef.repository}:${branchRef.branch_name}`
    : undefined;
}

export function formatPullRequestRef(pullRequestRef?: PullRequestRef) {
  return pullRequestRef
    ? `${pullRequestRef.provider}:${pullRequestRef.repository}:${pullRequestRef.pull_request_number}`
    : undefined;
}
