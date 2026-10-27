/**
 * Unit tests for the live HTTP task gateway.
 *
 * The suite validates success parsing plus shared-error mapping without
 * reaching a real backend.
 */
import type { CreateTaskRequest } from '../../domain/task';
import { TaskGatewayError } from '../../ports/task-slice-gateway';
import { createHttpTaskGateway } from './http-task-gateway';

const taskEnvelope = `{
  "data": {
    "task": {
      "created_at": "2026-04-13T00:00:00.000Z",
      "id": "task-1",
      "origin": {
        "issue_ref": {
          "issue_number": 42,
          "provider": "github",
          "repository": "acme/widgets"
        },
        "metadata": {
          "assignees": [],
          "labels": [],
          "title": "Stabilize the transport contract"
        },
        "type": "issue"
      },
      "state": "draft",
      "updated_at": "2026-04-13T00:00:00.000Z"
    }
  },
  "error": null,
  "metadata": {
    "request_id": "req-123",
    "timestamp": "2026-04-13T00:00:00.000Z",
    "version": "v1"
  },
  "success": true
}`;

/** Returns a fetch mock that resolves with the supplied response. */
function createFetchForResponse(response: Response): typeof fetch {
  return vi.fn<typeof fetch>().mockResolvedValue(response);
}

/** Builds the gateway with a fetch mock fixed to the supplied response. */
function createGatewayWithResponse(response: Response) {
  return createHttpTaskGateway('/api/v1', createFetchForResponse(response));
}

/**
 * Builds a gateway with one literal replacement in the task envelope.
 *
 * @example
 * `createGatewayWithTaskField('"issue_number": 42', '"issue_number": 0')`
 * serves an envelope containing issue number zero.
 */
function createGatewayWithTaskField(original: string, replacement: string) {
  return createGatewayWithResponse(
    new Response(taskEnvelope.replace(original, replacement)),
  );
}

describe('http task gateway', () => {
  it('parses successful task detail responses', async () => {
    const gateway = createGatewayWithResponse(new Response(taskEnvelope));

    await expect(gateway.getTask('task-1')).resolves.toMatchObject({
      id: 'task-1',
      state: 'draft',
    });
  });

  it('preserves task fields added by the API outside the current model', async () => {
    const responseBody = taskEnvelope.replace(
      '"id": "task-1"',
      '"id": "task-1",\n      "upstream_extension": "retained"',
    );
    const gateway = createGatewayWithResponse(new Response(responseBody));

    await expect(gateway.getTask('task-1')).resolves.toMatchObject({
      upstream_extension: 'retained',
    });
  });

  it('escapes task identifiers in task path segments', async () => {
    const fetchFn = vi
      .fn<typeof fetch>()
      .mockResolvedValueOnce(new Response(taskEnvelope))
      .mockResolvedValueOnce(new Response(taskEnvelope));
    const gateway = createHttpTaskGateway('/api/v1', fetchFn);

    await gateway.getTask('task/with reserved?characters');
    await gateway.transitionTask('task/with reserved?characters', 'done');

    expect(fetchFn.mock.calls.map((call) => call[0])).toEqual([
      '/api/v1/tasks/task%2Fwith%20reserved%3Fcharacters',
      '/api/v1/tasks/task%2Fwith%20reserved%3Fcharacters/state',
    ]);
    expect(fetchFn).toHaveBeenNthCalledWith(
      2,
      '/api/v1/tasks/task%2Fwith%20reserved%3Fcharacters/state',
      expect.objectContaining({ method: 'PUT' }),
    );
  });

  it.each([
    {
      body: taskEnvelope.replace(
        '"request_id": "req-123",\n    "timestamp": "2026-04-13T00:00:00.000Z"',
        '"request_id": "req-123",\n    "timestamp": ""',
      ),
      name: 'metadata is incomplete',
    },
    {
      body: taskEnvelope.replace('"version": "v1"', '"version": "v2"'),
      name: 'metadata version is unsupported',
    },
  ])('rejects a 200 envelope when $name', async ({ body }) => {
    const gateway = createGatewayWithResponse(new Response(body));

    await expect(gateway.getTask('task-1')).rejects.toEqual(
      new TaskGatewayError(
        'unavailable',
        'The task API returned an invalid success response.',
      ),
    );
  });

  it('maps shared not-found responses to the task gateway error', async () => {
    const response = Response.json(
      {
        code: 'not_found',
        details: { reason: 'task_not_found' },
        message: 'task task-9 was not found',
        traceId: 'trace-123',
      },
      { status: 404 },
    );
    const gateway = createGatewayWithResponse(response);

    await expect(gateway.getTask('task-9')).rejects.toEqual(
      new TaskGatewayError('not_found', 'task task-9 was not found'),
    );
  });

  it.each([
    {
      body: taskEnvelope.replace('"id": "task-1"', '"id": 123'),
      name: 'the nested task violates the slice contract shape',
    },
    {
      body: taskEnvelope.replace(
        '"created_at": "2026-04-13T00:00:00.000Z"',
        '"created_at": "2026-02-30T00:00:00.000Z"',
      ),
      name: 'the nested task has invalid timestamps',
    },
  ])('rejects a 200 envelope when $name', async ({ body }) => {
    const gateway = createGatewayWithResponse(new Response(body));

    await expect(gateway.getTask('task-shape')).rejects.toEqual(
      new TaskGatewayError(
        'unavailable',
        'The task API returned an invalid task shape.',
      ),
    );
  });

  it('maps invalid transition conflicts separately from validation failures', async () => {
    const response = Response.json(
      {
        code: 'conflict',
        details: {
          reason: 'invalid_task_transition',
          taskId: 'task-2',
        },
        message: 'invalid state transition for task task-2',
        traceId: 'trace-234',
      },
      { status: 409 },
    );
    const gateway = createGatewayWithResponse(response);

    await expect(gateway.transitionTask('task-2', 'done')).rejects.toEqual(
      new TaskGatewayError(
        'conflict',
        'invalid state transition for task task-2',
      ),
    );
  });

  it('sends the create request with an idempotency key', async () => {
    const fetchFn = createFetchForResponse(new Response(taskEnvelope));
    const gateway = createHttpTaskGateway('/api/v1', fetchFn);
    const request = {
      provider: 'github',
      repository: 'acme/widgets',
      issue_number: 42,
      title: 'A task',
    } satisfies CreateTaskRequest;

    await gateway.createTask(request);

    expect(fetchFn).toHaveBeenCalledWith(
      '/api/v1/tasks',
      expect.objectContaining({
        method: 'POST',
        body: JSON.stringify(request),
        headers: {
          'Content-Type': 'application/json',
          'Idempotency-Key': expect.stringMatching(
            /^[0-9a-f]{8}-[0-9a-f]{4}-4[0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}$/i,
          ),
        },
      }),
    );
  });

  it.each([
    { status: 400, kind: 'validation' },
    { status: 401, kind: 'unauthorized' },
    { status: 403, kind: 'unauthorized' },
    { status: 404, kind: 'not_found' },
    { status: 409, kind: 'conflict' },
    { status: 503, kind: 'unavailable' },
  ])('maps HTTP $status to $kind', async ({ status, kind }) => {
    const gateway = createGatewayWithResponse(
      Response.json({ message: 'API error' }, { status }),
    );

    await expect(gateway.getTask('task-1')).rejects.toMatchObject({
      kind,
      message: 'API error',
    });
  });

  it('reports malformed success JSON', async () => {
    const gateway = createGatewayWithResponse(new Response('{'));

    await expect(gateway.getTask('task-1')).rejects.toEqual(
      new TaskGatewayError(
        'unavailable',
        'The task API returned malformed JSON.',
      ),
    );
  });

  it('maps a rejected fetch to unavailable', async () => {
    const fetchFn = vi
      .fn<typeof fetch>()
      .mockRejectedValue(new Error('offline'));
    const gateway = createHttpTaskGateway('/api/v1', fetchFn);

    await expect(gateway.getTask('task-1')).rejects.toEqual(
      new TaskGatewayError('unavailable', 'The task API could not be reached.'),
    );
  });

  it.each([
    { body: '{', name: 'malformed JSON' },
    { body: '{"message": 42}', name: 'an invalid error shape' },
  ])('falls back to the HTTP status for $name', async ({ body }) => {
    const gateway = createGatewayWithResponse(
      new Response(body, { status: 503 }),
    );

    await expect(gateway.getTask('task-1')).rejects.toEqual(
      new TaskGatewayError('unavailable', 'The task API returned HTTP 503.'),
    );
  });

  it('normalizes null optional references to absence', async () => {
    const responseBody = taskEnvelope.replace(
      '"id": "task-1"',
      '"id": "task-1", "branch_ref": null, "pull_request_ref": null',
    );
    const task = await createGatewayWithResponse(
      new Response(responseBody),
    ).getTask('task-1');

    expect(task.branch_ref).toBeUndefined();
    expect(task.pull_request_ref).toBeUndefined();
  });

  it.each(
    Array.from({ length: 12 }, (_, index) => ({
      sequence: index,
      text: `extension-${index}`,
      nested: { enabled: index % 2 === 0 },
    })),
  )('retains generated task extension $sequence', async (extension) => {
    const responseBody = taskEnvelope.replace(
      '"id": "task-1"',
      `"id": "task-1", "extension": ${JSON.stringify(extension)}`,
    );
    const gateway = createGatewayWithResponse(new Response(responseBody));

    await expect(gateway.getTask('task-1')).resolves.toMatchObject({
      extension,
    });
  });

  it.each([
    '2024-02-29T23:59:59Z',
    '2026-01-01T00:00:00+23:59',
    '2026-12-31T23:59:59.123456-23:59',
    ...Array.from(
      { length: 12 },
      (_, index) => `2026-${String(index + 1).padStart(2, '0')}-01T12:30:45Z`,
    ),
  ])('accepts valid calendar timestamp %s', async (timestamp) => {
    const gateway = createGatewayWithTaskField(
      '"created_at": "2026-04-13T00:00:00.000Z"',
      `"created_at": "${timestamp}"`,
    );

    await expect(gateway.getTask('task-1')).resolves.toMatchObject({
      created_at: timestamp,
    });
  });

  it.each([
    ...[0, -1, -42, 1.5, 9_007_199_254_740_992].map((issueNumber) => ({
      name: `issue number ${issueNumber}`,
      original: '"issue_number": 42',
      replacement: `"issue_number": ${issueNumber}`,
    })),
    ...[
      '2023-02-29T00:00:00Z',
      '2026-04-31T00:00:00Z',
      '2026-01-01T24:00:00Z',
      '2026-01-01T23:60:00Z',
      '2026-01-01T23:59:60Z',
      '2026-01-01T00:00:00+24:00',
      '2026-01-01T00:00:00-01:60',
      ...Array.from(
        { length: 12 },
        (_, index) => `2026-${String(index + 1).padStart(2, '0')}-32T12:30:45Z`,
      ),
    ].map((timestamp) => ({
      name: `calendar timestamp ${timestamp}`,
      original: '"created_at": "2026-04-13T00:00:00.000Z"',
      replacement: `"created_at": "${timestamp}"`,
    })),
  ])('rejects invalid $name', async ({ original, replacement }) => {
    const gateway = createGatewayWithTaskField(original, replacement);

    await expect(gateway.getTask('task-1')).rejects.toEqual(
      new TaskGatewayError(
        'unavailable',
        'The task API returned an invalid task shape.',
      ),
    );
  });
});
