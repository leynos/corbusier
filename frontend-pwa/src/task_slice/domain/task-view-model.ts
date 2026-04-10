import type {
  BranchRef,
  PullRequestRef,
  TaskOrigin,
  TaskState,
  TaskStateViewModel,
} from './task';

type TaskStateMessageKey = `task.state.${TaskState}`;

export function formatIssueOrigin(origin: TaskOrigin) {
  const issueRef = origin.issue_ref;
  return `${issueRef.provider}/${issueRef.repository}/#${issueRef.issue_number}`;
}

export function formatTaskState(
  state: TaskState,
  t: (key: TaskStateMessageKey) => string,
): TaskStateViewModel {
  switch (state) {
    case 'draft':
      return { label: t('task.state.draft'), tone: 'steady' };
    case 'in_progress':
      return { label: t('task.state.in_progress'), tone: 'steady' };
    case 'in_review':
      return { label: t('task.state.in_review'), tone: 'steady' };
    case 'paused':
      return { label: t('task.state.paused'), tone: 'warning' };
    case 'done':
      return { label: t('task.state.done'), tone: 'steady' };
    case 'abandoned':
      return { label: t('task.state.abandoned'), tone: 'warning' };
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
