/**
 * Implement the live HTTP task gateway used by the frontend slice.
 *
 * This adapter consumes the stabilized Corbusier task contract while keeping
 * transport parsing and error mapping inside the adapter boundary.
 */
import type {
  IssueProvider,
  Task,
  TaskOrigin,
  TaskState,
} from '../../domain/task';
import {
  TaskGatewayError,
  type TaskGatewayErrorKind,
  type TaskSliceGateway,
} from '../../ports/task-slice-gateway';

interface ApiMetadata {
  request_id: string;
  timestamp: string;
  version: string;
}

interface ApiEnvelope<T> {
  data: T;
  error: null;
  metadata: ApiMetadata;
  success: true;
}

interface TaskEnvelope {
  task: Task;
}

interface SharedErrorResponse {
  code: string;
  details?: Record<string, unknown>;
  message: string;
  traceId?: string;
}

const TASK_STATES = new Set<TaskState>([
  'draft',
  'in_progress',
  'in_review',
  'paused',
  'done',
  'abandoned',
]);

const INVALID_TASK_SHAPE_MESSAGE =
  'The task API returned an invalid task shape.' as const;

function taskShapeGatewayError(): TaskGatewayError {
  return new TaskGatewayError('unavailable', INVALID_TASK_SHAPE_MESSAGE);
}

function isPlainRecord(value: unknown): value is Record<string, unknown> {
  return Boolean(value && typeof value === 'object' && !Array.isArray(value));
}

function isIssueProvider(value: unknown): value is IssueProvider {
  return value === 'github' || value === 'gitlab';
}

function requireNonEmptyString(
  raw: Record<string, unknown>,
  field: string,
): string {
  const value = raw[field];
  if (typeof value !== 'string' || value.length === 0) {
    throw taskShapeGatewayError();
  }
  return value;
}

function requireStringArray(raw: Record<string, unknown>, field: string): void {
  const value = raw[field];
  if (!Array.isArray(value) || value.some((item) => typeof item !== 'string')) {
    throw taskShapeGatewayError();
  }
}

function validateIssueSnapshot(raw: unknown): void {
  if (!isPlainRecord(raw)) throw taskShapeGatewayError();
  requireNonEmptyString(raw, 'title');
  requireStringArray(raw, 'labels');
  requireStringArray(raw, 'assignees');
  if (raw.description !== undefined && typeof raw.description !== 'string') {
    throw taskShapeGatewayError();
  }
  if (raw.milestone !== undefined && typeof raw.milestone !== 'string') {
    throw taskShapeGatewayError();
  }
}

function validateIssueRef(raw: unknown): void {
  if (!isPlainRecord(raw)) throw taskShapeGatewayError();
  if (!isIssueProvider(raw.provider)) throw taskShapeGatewayError();
  requireNonEmptyString(raw, 'repository');
  const num = raw.issue_number;
  if (typeof num !== 'number' || !Number.isInteger(num) || num < 0) {
    throw taskShapeGatewayError();
  }
}

function validateTaskOrigin(raw: unknown): asserts raw is TaskOrigin {
  if (!isPlainRecord(raw)) throw taskShapeGatewayError();
  if (raw.type !== 'issue') throw taskShapeGatewayError();
  validateIssueRef(raw.issue_ref);
  validateIssueSnapshot(raw.metadata);
}

function validateOptionalBranchRef(raw: unknown): void {
  if (!isPlainRecord(raw)) throw taskShapeGatewayError();
  if (!isIssueProvider(raw.provider)) throw taskShapeGatewayError();
  requireNonEmptyString(raw, 'repository');
  requireNonEmptyString(raw, 'branch_name');
}

function validateOptionalPullRequestRef(raw: unknown): void {
  if (!isPlainRecord(raw)) throw taskShapeGatewayError();
  if (!isIssueProvider(raw.provider)) throw taskShapeGatewayError();
  requireNonEmptyString(raw, 'repository');
  const n = raw.pull_request_number;
  if (typeof n !== 'number' || !Number.isInteger(n) || n < 1) {
    throw taskShapeGatewayError();
  }
}

function validateParsedTaskEnvelopeTask(task: unknown): Task {
  if (!isPlainRecord(task)) throw taskShapeGatewayError();
  requireNonEmptyString(task, 'id');
  if (
    typeof task.state !== 'string' ||
    !(TASK_STATES as ReadonlySet<string>).has(task.state)
  ) {
    throw taskShapeGatewayError();
  }
  validateTaskOrigin(task.origin);
  if (task.branch_ref !== undefined && task.branch_ref !== null) {
    validateOptionalBranchRef(task.branch_ref);
  }
  if (task.pull_request_ref !== undefined && task.pull_request_ref !== null) {
    validateOptionalPullRequestRef(task.pull_request_ref);
  }
  const createdAt = task.created_at;
  const updatedAt = task.updated_at;
  if (typeof createdAt !== 'string' || createdAt.length === 0) {
    throw taskShapeGatewayError();
  }
  if (typeof updatedAt !== 'string' || updatedAt.length === 0) {
    throw taskShapeGatewayError();
  }
  return task as unknown as Task;
}

export function createHttpTaskGateway(
  baseUrl: string = '/api/v1',
  fetchFn: typeof fetch = fetch,
): TaskSliceGateway {
  return {
    createTask(request) {
      return sendTaskRequest(fetchFn, `${baseUrl}/tasks`, {
        body: JSON.stringify(request),
        headers: {
          'Content-Type': 'application/json',
          'Idempotency-Key': crypto.randomUUID(),
        },
        method: 'POST',
      });
    },
    getTask(taskId) {
      return sendTaskRequest(fetchFn, `${baseUrl}/tasks/${taskId}`);
    },
    transitionTask(taskId, targetState) {
      return sendTaskRequest(fetchFn, `${baseUrl}/tasks/${taskId}/state`, {
        body: JSON.stringify({ state: targetState }),
        headers: {
          'Content-Type': 'application/json',
          'Idempotency-Key': crypto.randomUUID(),
        },
      });
    },
  };
}

async function sendTaskRequest(
  fetchFn: typeof fetch,
  resource: string,
  init?: RequestInit,
): Promise<Task> {
  let response: Response;
  try {
    response = await fetchFn(resource, init);
  } catch {
    throw new TaskGatewayError(
      'unavailable',
      'The task API could not be reached.',
    );
  }

  if (!response.ok) {
    throw await readGatewayError(response);
  }

  const envelope = await parseSuccessEnvelope(response);
  const taskCandidate = envelope.data?.task as unknown;
  if (taskCandidate === undefined || taskCandidate === null) {
    throw new TaskGatewayError(
      'unavailable',
      'The task API returned an invalid response.',
    );
  }

  return validateParsedTaskEnvelopeTask(taskCandidate);
}

async function parseSuccessEnvelope(
  response: Response,
): Promise<ApiEnvelope<TaskEnvelope>> {
  try {
    return (await response.json()) as ApiEnvelope<TaskEnvelope>;
  } catch {
    throw new TaskGatewayError(
      'unavailable',
      'The task API returned malformed JSON.',
    );
  }
}

async function readGatewayError(response: Response): Promise<TaskGatewayError> {
  let body: SharedErrorResponse | null = null;

  try {
    body = (await response.json()) as SharedErrorResponse;
  } catch {
    return new TaskGatewayError(
      'unavailable',
      `The task API returned HTTP ${response.status}.`,
    );
  }

  return new TaskGatewayError(
    mapErrorKind(response.status, body),
    body.message || `The task API returned HTTP ${response.status}.`,
  );

  function mapErrorKind(
    statusCode: number,
    errorBody: SharedErrorResponse,
  ): TaskGatewayErrorKind {
    const reason = errorBody.details?.reason;
    if (typeof reason === 'string' && reason === 'invalid_task_transition') {
      return 'conflict';
    }

    return classifyErrorKind(statusCode);
  }

  function classifyErrorKind(statusCode: number): TaskGatewayErrorKind {
    if (statusCode === 401 || statusCode === 403) {
      return 'unauthorized';
    }
    if (statusCode === 404) {
      return 'not_found';
    }
    if (statusCode === 409) {
      return 'conflict';
    }
    if (statusCode === 400) {
      return 'validation';
    }
    return 'unavailable';
  }
}
