/**
 * Define task-create draft state and conversion helpers for the route layer.
 *
 * `TaskCreateDraft` models form-local strings while this module validates and
 * converts them into the domain `CreateTaskRequest` contract.
 */
import type { CreateTaskRequest, IssueProvider } from './task';

/**
 * Form-local draft of a task-create request; every field is a raw string
 * so controlled inputs can hold in-progress, possibly invalid, text.
 */
export interface TaskCreateDraft {
  /** Issue provider selected for the new task. */
  provider: IssueProvider;
  /** Owner-qualified repository text, for example `owner/repository`. */
  repository: string;
  /** Raw issue number text, parsed and range-checked on validation. */
  issueNumber: string;
  /** Task title; required, so validation rejects blank text. */
  title: string;
  /** Task description; optional, so an empty string is valid. */
  description: string;
  /** Comma-delimited labels, split by {@link splitDelimitedValues}. */
  labels: string;
  /** Comma-delimited assignees, split by {@link splitDelimitedValues}. */
  assignees: string;
  /** Milestone name; optional, so an empty string is valid. */
  milestone: string;
}

/** A key of {@link TaskCreateDraft}, identifying one form field. */
export type TaskCreateField = keyof TaskCreateDraft;
/** Per-field validation messages, present only for invalid fields. */
export type TaskCreateErrors = Partial<Record<TaskCreateField, string>>;
/** Issue providers offered in the task-create form. */
export const SUPPORTED_PROVIDERS: readonly IssueProvider[] = [
  'github',
  'gitlab',
];
const ERROR_PRECEDENCE: readonly TaskCreateField[] = [
  'provider',
  'repository',
  'issueNumber',
  'title',
  'description',
  'labels',
  'assignees',
  'milestone',
];

/** Empty draft used to seed the task-create form on first render. */
export const initialTaskCreateDraft: TaskCreateDraft = {
  provider: SUPPORTED_PROVIDERS[0],
  repository: '',
  issueNumber: '',
  title: '',
  description: '',
  labels: '',
  assignees: '',
  milestone: '',
};

/**
 * Split a comma-delimited form field into trimmed, non-empty entries.
 */
export function splitDelimitedValues(raw: string) {
  return raw
    .split(',')
    .map((value) => value.trim())
    .filter(Boolean);
}

/**
 * Validate a task-create draft, returning a message per invalid field.
 * An empty result indicates the draft is ready for submission.
 */
export function validateTaskCreateDraft(
  draft: TaskCreateDraft,
): TaskCreateErrors {
  const errors: TaskCreateErrors = {};

  if (!SUPPORTED_PROVIDERS.includes(draft.provider)) {
    errors.provider = 'Choose a supported provider.';
  }

  if (!/^[^/\s]+\/[^/\s]+$/.test(draft.repository.trim())) {
    errors.repository = 'Use the repository format owner/repository.';
  }

  const issueNumber = Number(draft.issueNumber);
  if (!Number.isInteger(issueNumber) || issueNumber <= 0) {
    errors.issueNumber = 'Issue number must be a positive integer.';
  }

  if (draft.title.trim().length === 0) {
    errors.title = 'Title is required.';
  }

  return errors;
}

/**
 * Assert that a draft satisfies the task-create validation rules.
 *
 * @param draft - Draft values collected from the task-create form.
 * @throws {Error} When the draft fails validation.
 */
function assertValidDraft(draft: TaskCreateDraft): void {
  const errors = validateTaskCreateDraft(draft);
  let firstError: string | undefined;
  for (const field of ERROR_PRECEDENCE) {
    firstError = errors[field];
    if (firstError !== undefined) {
      break;
    }
  }

  if (firstError !== undefined) {
    throw new Error(firstError);
  }
}

/**
 * Convert a validated draft into the domain `CreateTaskRequest` shape,
 * trimming strings and dropping optional fields left blank.
 *
 * @throws {Error} When the draft fails validation.
 */
export function toCreateTaskRequest(draft: TaskCreateDraft): CreateTaskRequest {
  assertValidDraft(draft);

  return {
    provider: draft.provider,
    repository: draft.repository.trim(),
    issue_number: Number(draft.issueNumber),
    title: draft.title.trim(),
    description: normalizeOptionalString(draft.description),
    labels: normalizeOptionalList(draft.labels),
    assignees: normalizeOptionalList(draft.assignees),
    milestone: normalizeOptionalString(draft.milestone),
  };
}

function normalizeOptionalString(value: string) {
  const normalized = value.trim();
  return normalized.length > 0 ? normalized : undefined;
}

function normalizeOptionalList(value: string) {
  const normalized = splitDelimitedValues(value);
  return normalized.length > 0 ? normalized : undefined;
}
