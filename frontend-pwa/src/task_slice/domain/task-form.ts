/**
 * Define task-create draft state and conversion helpers for the route layer.
 *
 * `TaskCreateDraft` models form-local strings while this module validates and
 * converts them into the domain `CreateTaskRequest` contract.
 */
import type { CreateTaskRequest, IssueProvider } from './task';

export interface TaskCreateDraft {
  provider: IssueProvider;
  repository: string;
  issueNumber: string;
  title: string;
  description: string;
  labels: string;
  assignees: string;
  milestone: string;
}

export type TaskCreateField = keyof TaskCreateDraft;
export type TaskCreateErrors = Partial<Record<TaskCreateField, string>>;
export const SUPPORTED_PROVIDERS: readonly IssueProvider[] = [
  'github',
  'gitlab',
];

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

export function splitDelimitedValues(raw: string) {
  return raw
    .split(',')
    .map((value) => value.trim())
    .filter(Boolean);
}

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
  const [firstError] = Object.values(errors);
  if (firstError !== undefined) {
    throw new Error(firstError);
  }
}

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
