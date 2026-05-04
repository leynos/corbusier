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

/**
 * Returns true when `value` is a finite integer ≥ `min`.
 * Narrows the type to `number` for the caller.
 */
function isIntegerAtLeast(value: unknown, min: number): value is number {
  return typeof value === 'number' && Number.isInteger(value) && value >= min;
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
  if (!isIntegerAtLeast(raw.issue_number, 0)) throw taskShapeGatewayError();
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
  if (!isIntegerAtLeast(raw.pull_request_number, 1))
    throw taskShapeGatewayError();
}

function requireValidTaskState(task: Record<string, unknown>): void {
  if (
    typeof task.state !== 'string' ||
    !(TASK_STATES as ReadonlySet<string>).has(task.state)
  ) {
    throw taskShapeGatewayError();
  }
}

function validateOptionalRefs(task: Record<string, unknown>): void {
  if (task.branch_ref !== undefined && task.branch_ref !== null) {
    validateOptionalBranchRef(task.branch_ref);
  }
  if (task.pull_request_ref !== undefined && task.pull_request_ref !== null) {
    validateOptionalPullRequestRef(task.pull_request_ref);
  }
}

function validateTimestampFields(task: Record<string, unknown>): void {
  const createdAt = task.created_at;
  const updatedAt = task.updated_at;
  if (typeof createdAt !== 'string' || createdAt.length === 0) {
    throw taskShapeGatewayError();
  }
  if (typeof updatedAt !== 'string' || updatedAt.length === 0) {
    throw taskShapeGatewayError();
  }
}

function validateParsedTaskEnvelopeTask(task: unknown): Task {
  if (!isPlainRecord(task)) throw taskShapeGatewayError();
  requireNonEmptyString(task, 'id');
  requireValidTaskState(task);
  validateTaskOrigin(task.origin);
  validateOptionalRefs(task);
  validateTimestampFields(task);
  return task as unknown as Task;
}

/**
 * Builds the RequestInit for state-mutating task requests (POST / PUT).
 * Centralises the repeated Content-Type + Idempotency-Key header pair and
 * JSON serialisation so that createTask and transitionTask stay DRY.
 */
function mutationInit(body: unknown, method: 'POST' | 'PUT'): RequestInit {
  return {
    body: JSON.stringify(body),
    headers: {
      'Content-Type': 'application/json',
      'Idempotency-Key': crypto.randomUUID(),
    },
    method,
  };
}

export function createHttpTaskGateway(
  baseUrl: string = '/api/v1',
  fetchFn: typeof fetch = fetch,
): TaskSliceGateway {
  return {
    createTask(request) {
      return sendTaskRequest(
        fetchFn,
        `${baseUrl}/tasks`,
        mutationInit(request, 'POST'),
      );
    },
    getTask(taskId) {
      return sendTaskRequest(fetchFn, `${baseUrl}/tasks/${taskId}`);
    },
    transitionTask(taskId, targetState) {
      return sendTaskRequest(
        fetchFn,
        `${baseUrl}/tasks/${taskId}/state`,
        mutationInit({ state: targetState }, 'PUT'),
      );
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
  const HTTP_STATUS_TO_ERROR_KIND: Readonly<
    Partial<Record<number, TaskGatewayErrorKind>>
  > = {
    400: 'validation',
    401: 'unauthorized',
    403: 'unauthorized',
    404: 'not_found',
    409: 'conflict',
  } as const;

  function classifyErrorKind(statusCode: number): TaskGatewayErrorKind {
    return HTTP_STATUS_TO_ERROR_KIND[statusCode] ?? 'unavailable';
  }

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
}
