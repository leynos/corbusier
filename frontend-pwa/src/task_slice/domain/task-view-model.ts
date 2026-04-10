/**
 * Format task domain values into UI-friendly view models.
 *
 * This module keeps route and component code small by centralizing formatting
 * for task state labels, timestamps, and branch or pull-request references.
 */
import type {
  BranchRef,
  PullRequestRef,
  TaskOrigin,
  TaskState,
  TaskStateViewModel,
} from './task';

type TaskStateMessageKey = `task.state.${TaskState}`;
type TaskStateTone = TaskStateViewModel['tone'];

const toneMap: Record<TaskState, TaskStateTone> = {
  draft: 'steady',
  in_progress: 'steady',
  in_review: 'steady',
  paused: 'warning',
  done: 'steady',
  abandoned: 'warning',
};

export function formatIssueOrigin(origin: TaskOrigin) {
  const issueRef = origin.issue_ref;
  return `${issueRef.provider}/${issueRef.repository}/#${issueRef.issue_number}`;
}

export function formatTaskState(
  state: TaskState,
  t: (key: TaskStateMessageKey) => string,
): TaskStateViewModel {
  return { label: t(`task.state.${state}`), tone: toneMap[state] };
}

export function formatTimestamp(value: string, locale = 'en-GB') {
  const date = new Date(value);
  if (Number.isNaN(date.getTime())) {
    return '';
  }

  return new Intl.DateTimeFormat(locale, {
    dateStyle: 'medium',
    timeStyle: 'short',
    timeZone: 'UTC',
  }).format(date);
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
