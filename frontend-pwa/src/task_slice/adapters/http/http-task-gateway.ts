/**
 * Implement the live HTTP task gateway used by the frontend slice.
 *
 * This adapter consumes the stabilized Corbusier task contract while keeping
 * transport parsing and error mapping inside the adapter boundary.
 */
import { z } from 'zod';

import type { CreateTaskRequest, Task, TaskState } from '../../domain/task';
import {
  TaskGatewayError,
  type TaskGatewayErrorKind,
  type TaskSliceGateway,
} from '../../ports/task-slice-gateway';

const INVALID_TASK_RESPONSE_MESSAGE =
  'The task API returned an invalid task shape.';

const INVALID_SUCCESS_ENVELOPE_MESSAGE =
  'The task API returned an invalid success response.';

const CONTRACT_VERSION = 'v1' as const;
const nonEmptyStringSchema = z.string().min(1);
const positiveIntegerSchema = z.number().int().min(1);
const stringArraySchema = z.array(z.string());
const timestampSchema = z.string().refine(isValidTimestamp);
const providerSchema = z.enum(['github', 'gitlab']);

const issueSnapshotSchema = z.looseObject({
  title: nonEmptyStringSchema,
  description: z.string().optional(),
  labels: stringArraySchema,
  assignees: stringArraySchema,
  milestone: z.string().optional(),
});

// The API may add fields independently; validate known fields and retain
// extensions for callers that can understand them.
const taskSchema = z.looseObject({
  id: nonEmptyStringSchema,
  origin: z.looseObject({
    type: z.literal('issue'),
    issue_ref: z.looseObject({
      provider: providerSchema,
      repository: nonEmptyStringSchema,
      issue_number: positiveIntegerSchema,
    }),
    metadata: issueSnapshotSchema,
  }),
  branch_ref: z
    .looseObject({
      provider: providerSchema,
      repository: nonEmptyStringSchema,
      branch_name: nonEmptyStringSchema,
    })
    .nullish()
    .transform((reference) => reference || undefined),
  pull_request_ref: z
    .looseObject({
      provider: providerSchema,
      repository: nonEmptyStringSchema,
      pull_request_number: positiveIntegerSchema,
    })
    .nullish()
    .transform((reference) => reference || undefined),
  state: z.enum([
    'draft',
    'in_progress',
    'in_review',
    'paused',
    'done',
    'abandoned',
  ]),
  created_at: timestampSchema,
  updated_at: timestampSchema,
}) satisfies z.ZodType<Task>;

const successEnvelopeSchema = z.object({
  data: z.object({ task: z.unknown() }),
  error: z.null(),
  metadata: z.object({
    request_id: nonEmptyStringSchema,
    timestamp: timestampSchema,
    version: z.literal(CONTRACT_VERSION),
  }),
  success: z.literal(true),
});

const sharedErrorResponseSchema = z.object({
  code: z.string().optional(),
  details: z.unknown().optional(),
  message: z.string(),
  traceId: z.string().optional(),
});

const invalidTaskTransitionSchema = z.object({
  reason: z.literal('invalid_task_transition'),
});

/**
 * Create the gateway error used when a task response violates its contract.
 * Callers can report the failure as an unavailable API response.
 *
 * @returns A stable error for malformed task data.
 *
 * @example
 * throw taskContractGatewayError();
 */
function taskContractGatewayError(): TaskGatewayError {
  return new TaskGatewayError('unavailable', INVALID_TASK_RESPONSE_MESSAGE);
}

/**
 * Builds the RequestInit for state-mutating task requests (POST / PUT).
 * Centralizes the repeated Content-Type + Idempotency-Key header pair and
 * JSON serialization so that createTask and transitionTask stay DRY.
 */
function mutationInit(
  body: CreateTaskRequest | { state: TaskState },
  method: 'POST' | 'PUT',
): RequestInit {
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
 * @example
 * `createHttpTaskGateway('/api/v1').getTask('task-1')` fetches task `task-1`.
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

/**
 * Fetch one task resource and decode the task API's success envelope.
 * Network failures and invalid responses reject with `TaskGatewayError`.
 *
 * @param fetchFn - Fetch implementation, injectable for adapter tests.
 * @param resource - Task API URL to request.
 * @param init - Optional request method, headers, and body.
 * @returns The validated task from the response.
 *
 * @example
 * await sendTaskRequest(fetch, '/tasks/123'); // Resolves to the validated task.
 */
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

  return parseSuccessEnvelope(response);
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

/**
 * Parse the components of an ISO timestamp with an explicit UTC offset.
 * Returns `undefined` when the string does not match the timestamp syntax;
 * calendar and range checks are performed by `isValidTimestamp`.
 *
 * @param value - Timestamp string to parse.
 * @returns Parsed components, or `undefined` when the syntax does not match.
 *
 * @example
 * parseTimestampGroups('2024-02-29T12:30:00Z'); // Returns the parsed components.
 */
function parseTimestampGroups(value: string): ParsedTimestamp | undefined {
  const match = TIMESTAMP_RE.exec(value);
  const groups = match?.groups;
  if (!groups) return undefined;
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

/**
 * Check that a timestamp has valid syntax, calendar values, time, and offset.
 *
 * @param value - Timestamp string to validate.
 * @returns `true` only for a valid timestamp with an explicit offset.
 *
 * @example
 * isValidTimestamp('2024-02-29T12:30:00Z'); // Returns true.
 */
function isValidTimestamp(value: string): boolean {
  const parts = parseTimestampGroups(value);
  if (parts === undefined) return false;
  return (
    isValidTimeOfDay(parts.hour, parts.minute, parts.second) &&
    isValidOffset(parts.offset) &&
    matchesCalendarDate(parts)
  );
}

/**
 * Decode and validate a successful task API response.
 * Malformed JSON, an invalid envelope, or an invalid task rejects with a
 * `TaskGatewayError` of kind `unavailable`.
 *
 * @param response - Successful HTTP response from the task API.
 * @returns The validated task in `data.data.task`.
 *
 * @example
 * await parseSuccessEnvelope(response); // Resolves to the response task.
 */
async function parseSuccessEnvelope(response: Response): Promise<Task> {
  let responseBody: unknown;
  try {
    responseBody = await response.json();
  } catch {
    throw new TaskGatewayError(
      'unavailable',
      'The task API returned malformed JSON.',
    );
  }

  const envelope = successEnvelopeSchema.safeParse(responseBody);
  if (!envelope.success) {
    throw new TaskGatewayError('unavailable', INVALID_SUCCESS_ENVELOPE_MESSAGE);
  }

  const task = taskSchema.safeParse(envelope.data.data.task);
  if (!task.success) throw taskContractGatewayError();
  return task.data;
}

/**
 * Convert a non-success HTTP response into the gateway's typed error.
 * Invalid or unreadable error bodies use the HTTP-status fallback and the
 * `unavailable` kind.
 *
 * @param response - Non-success response returned by the task API.
 * @returns A gateway error with the mapped kind and API or fallback message.
 *
 * @example
 * const error = await readGatewayError(response); // Inspect `error.kind` and `error.message`.
 */
async function readGatewayError(response: Response): Promise<TaskGatewayError> {
  const fallbackMessage = `The task API returned HTTP ${response.status}.`;

  let responseBody: unknown;
  try {
    responseBody = await response.json();
  } catch {
    return new TaskGatewayError('unavailable', fallbackMessage);
  }

  const errorBody = sharedErrorResponseSchema.safeParse(responseBody);
  if (!errorBody.success) {
    return new TaskGatewayError('unavailable', fallbackMessage);
  }

  const errorKind = invalidTaskTransitionSchema.safeParse(
    errorBody.data.details,
  ).success
    ? 'conflict'
    : (errorKindForStatus(response.status) ?? 'unavailable');

  return new TaskGatewayError(
    errorKind,
    errorBody.data.message || fallbackMessage,
  );
}

/**
 * Map task API status codes to their corresponding gateway error kinds.
 * Unknown statuses return `undefined` so callers can choose a fallback.
 *
 * @param status - HTTP status code from the task API.
 * @returns A mapped kind for known client errors, otherwise `undefined`.
 *
 * @example
 * errorKindForStatus(404); // Returns 'not_found'.
 */
function errorKindForStatus(status: number): TaskGatewayErrorKind | undefined {
  switch (status) {
    case 400: {
      return 'validation';
    }
    case 401:
    case 403: {
      return 'unauthorized';
    }
    case 404: {
      return 'not_found';
    }
    case 409: {
      return 'conflict';
    }
    default: {
      return undefined;
    }
  }
}
