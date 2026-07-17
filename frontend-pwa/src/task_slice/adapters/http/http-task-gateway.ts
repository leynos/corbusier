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

const INVALID_SUCCESS_ENVELOPE_MESSAGE =
  'The task API returned an invalid success response.' as const;

const CONTRACT_VERSION = 'v1' as const;

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

function requireOptionalString(
  raw: Record<string, unknown>,
  key: string,
): void {
  if (raw[key] !== undefined && typeof raw[key] !== 'string') {
    throw taskShapeGatewayError();
  }
}

function validateIssueSnapshot(raw: unknown): void {
  if (!isPlainRecord(raw)) throw taskShapeGatewayError();
  requireNonEmptyString(raw, 'title');
  requireStringArray(raw, 'labels');
  requireStringArray(raw, 'assignees');
  requireOptionalString(raw, 'description');
  requireOptionalString(raw, 'milestone');
}

/**
 * Returns true when `value` is a finite integer ≥ `min`.
 * Narrows the type to `number` for the caller.
 */
function isIntegerAtLeast(value: unknown, min: number): value is number {
  return typeof value === 'number' && Number.isInteger(value) && value >= min;
}

/** Throws if `record[field]` is not an integer ≥ `minValue`. */
function requireIntegerField(
  record: Record<string, unknown>,
  field: string,
  minValue: number,
): void {
  if (!isIntegerAtLeast(record[field], minValue)) {
    throw taskShapeGatewayError();
  }
}

function validateIssueRef(raw: unknown): void {
  if (!isPlainRecord(raw)) throw taskShapeGatewayError();
  if (!isIssueProvider(raw.provider)) throw taskShapeGatewayError();
  requireNonEmptyString(raw, 'repository');
  requireIntegerField(raw, 'issue_number', 1);
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
  requireIntegerField(raw, 'pull_request_number', 1);
}

function validateTaskState(task: Record<string, unknown>): void {
  if (
    typeof task.state !== 'string' ||
    !(TASK_STATES as ReadonlySet<string>).has(task.state)
  ) {
    throw taskShapeGatewayError();
  }
}

function validateOptionalRef(
  raw: unknown,
  validator: (v: unknown) => void,
): void {
  if (raw !== undefined && raw !== null) {
    validator(raw);
  }
}

function requireTimestampFields(task: Record<string, unknown>): void {
  if (!isValidTimestamp(task.created_at)) {
    throw taskShapeGatewayError();
  }
  if (!isValidTimestamp(task.updated_at)) {
    throw taskShapeGatewayError();
  }
}

function validateParsedTaskEnvelopeTask(task: unknown): Task {
  if (!isPlainRecord(task)) throw taskShapeGatewayError();
  requireNonEmptyString(task, 'id');
  validateTaskState(task);
  validateTaskOrigin(task.origin);
  validateOptionalRef(task.branch_ref, validateOptionalBranchRef);
  validateOptionalRef(task.pull_request_ref, validateOptionalPullRequestRef);
  requireTimestampFields(task);
  return task as unknown as Task;
}

/**
 * Builds the RequestInit for state-mutating task requests (POST / PUT).
 * Centralizes the repeated Content-Type + Idempotency-Key header pair and
 * JSON serialization so that createTask and transitionTask stay DRY.
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

/**
 * Build the `TaskSliceGateway` adapter backed by the live task API.
 *
 * @param baseUrl - API root; overridable for non-default deployments.
 * @param fetchFn - Injectable `fetch` implementation for testing.
 */
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
      return sendTaskRequest(
        fetchFn,
        `${baseUrl}/tasks/${encodeURIComponent(taskId)}`,
      );
    },
    transitionTask(taskId, targetState) {
      return sendTaskRequest(
        fetchFn,
        `${baseUrl}/tasks/${encodeURIComponent(taskId)}/state`,
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
  return validateParsedTaskEnvelopeTask(envelope.data.task);
}

function isValidEnvelopeShell(
  parsed: unknown,
): parsed is Record<string, unknown> {
  return (
    isPlainRecord(parsed) &&
    parsed.success === true &&
    parsed.error === null &&
    isPlainRecord(parsed.metadata) &&
    hasValidMetadata(parsed.metadata)
  );
}

function hasValidMetadata(metadata: Record<string, unknown>): boolean {
  return (
    isNonEmptyString(metadata.request_id) &&
    metadata.version === CONTRACT_VERSION &&
    isValidTimestamp(metadata.timestamp)
  );
}

function isNonEmptyString(value: unknown): value is string {
  return typeof value === 'string' && value.length > 0;
}

interface ParsedTimestamp {
  year: number;
  month: number;
  day: number;
  hour: number;
  minute: number;
  second: number;
  offset: string;
}

const TIMESTAMP_RE =
  /^(?<year>\d{4})-(?<month>\d{2})-(?<day>\d{2})T(?<hour>\d{2}):(?<minute>\d{2}):(?<second>\d{2})(?:\.(?<fraction>\d+))?(?<offset>Z|[+-]\d{2}:\d{2})$/;

function parseTimestampGroups(value: string): ParsedTimestamp | null {
  const match = TIMESTAMP_RE.exec(value);
  const groups = match?.groups;
  if (!groups) return null;
  return {
    year: Number(groups.year),
    month: Number(groups.month),
    day: Number(groups.day),
    hour: Number(groups.hour),
    minute: Number(groups.minute),
    second: Number(groups.second),
    offset: groups.offset,
  };
}

function isValidTimeOfDay(
  hour: number,
  minute: number,
  second: number,
): boolean {
  return hour <= 23 && minute <= 59 && second <= 59;
}

function isValidOffset(offset: string): boolean {
  if (offset === 'Z') return true;
  const offsetHour = Number(offset.slice(1, 3));
  const offsetMinute = Number(offset.slice(4, 6));
  return offsetHour <= 23 && offsetMinute <= 59;
}

function matchesCalendarDate(p: ParsedTimestamp): boolean {
  const date = new Date(
    Date.UTC(p.year, p.month - 1, p.day, p.hour, p.minute, p.second),
  );
  return (
    date.getUTCFullYear() === p.year &&
    date.getUTCMonth() === p.month - 1 &&
    date.getUTCDate() === p.day &&
    date.getUTCHours() === p.hour &&
    date.getUTCMinutes() === p.minute &&
    date.getUTCSeconds() === p.second
  );
}

function isValidTimestamp(value: unknown): value is string {
  if (!isNonEmptyString(value)) return false;
  const parts = parseTimestampGroups(value);
  if (parts === null) return false;
  return (
    isValidTimeOfDay(parts.hour, parts.minute, parts.second) &&
    isValidOffset(parts.offset) &&
    matchesCalendarDate(parts)
  );
}

function hasValidTaskPayload(data: unknown): boolean {
  return (
    isPlainRecord(data) &&
    'task' in data &&
    data.task !== null &&
    data.task !== undefined
  );
}

async function parseSuccessEnvelope(
  response: Response,
): Promise<ApiEnvelope<TaskEnvelope>> {
  let parsed: unknown;
  try {
    parsed = await response.json();
  } catch {
    throw new TaskGatewayError(
      'unavailable',
      'The task API returned malformed JSON.',
    );
  }

  if (!isValidEnvelopeShell(parsed)) {
    throw new TaskGatewayError('unavailable', INVALID_SUCCESS_ENVELOPE_MESSAGE);
  }
  if (!hasValidTaskPayload(parsed.data)) {
    throw new TaskGatewayError('unavailable', INVALID_SUCCESS_ENVELOPE_MESSAGE);
  }

  return parsed as unknown as ApiEnvelope<TaskEnvelope>;
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

  const fallbackMessage = `The task API returned HTTP ${response.status}.`;

  let body: unknown;
  try {
    body = await response.json();
  } catch {
    return new TaskGatewayError('unavailable', fallbackMessage);
  }

  if (!isPlainRecord(body) || typeof body.message !== 'string') {
    return new TaskGatewayError('unavailable', fallbackMessage);
  }

  const errorBody = body as unknown as SharedErrorResponse;

  return new TaskGatewayError(
    mapErrorKind(response.status, errorBody),
    errorBody.message || fallbackMessage,
  );
}
