/**
 * Implement the live HTTP task gateway used by the frontend slice.
 *
 * This adapter consumes the stabilized Corbusier task contract while keeping
 * transport parsing and error mapping inside the adapter boundary.
 */
import type { Task } from '../../domain/task';
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
        method: 'PUT',
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

  let envelope = await parseSuccessEnvelope(response);
  if (!envelope.data?.task) {
    throw new TaskGatewayError(
      'unavailable',
      'The task API returned an invalid response.',
    );
  }

  return envelope.data.task;
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
