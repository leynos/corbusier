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

/**
 * Render an issue origin as `provider/repository/#number` for display.
 */
export function formatIssueOrigin(origin: TaskOrigin) {
  const issueRef = origin.issue_ref;
  return `${issueRef.provider}/${issueRef.repository}/#${issueRef.issue_number}`;
}

/**
 * Combine a localized label with the tone assigned to `state`.
 *
 * @param t - Translation function keyed by `task.state.<state>`.
 */
export function formatTaskState(
  state: TaskState,
  t: (key: TaskStateMessageKey) => string,
): TaskStateViewModel {
  return { label: t(`task.state.${state}`), tone: toneMap[state] };
}

/**
 * Format an ISO 8601 timestamp for display, or an empty string if
 * `value` cannot be parsed as a date.
 */
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

/**
 * Render a branch reference as `provider:repository:branch`, or
 * `undefined` when no branch exists yet.
 */
export function formatBranchRef(branchRef?: BranchRef) {
  return branchRef
    ? `${branchRef.provider}:${branchRef.repository}:${branchRef.branch_name}`
    : undefined;
}

/**
 * Render a pull request reference as `provider:repository:number`, or
 * `undefined` when no pull request exists yet.
 */
export function formatPullRequestRef(pullRequestRef?: PullRequestRef) {
  return pullRequestRef
    ? `${pullRequestRef.provider}:${pullRequestRef.repository}:${pullRequestRef.pull_request_number}`
    : undefined;
}
