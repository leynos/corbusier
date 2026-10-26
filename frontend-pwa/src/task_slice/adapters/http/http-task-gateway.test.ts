/**
 * Unit tests for the live HTTP task gateway.
 *
 * The suite validates success parsing plus shared-error mapping without
 * reaching a real backend.
 */
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

function createFetchForResponse(response: Response): typeof fetch {
  return vi.fn<typeof fetch>().mockResolvedValue(response);
}

function createGatewayWithResponse(response: Response) {
  return createHttpTaskGateway('/api/v1', createFetchForResponse(response));
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
});
