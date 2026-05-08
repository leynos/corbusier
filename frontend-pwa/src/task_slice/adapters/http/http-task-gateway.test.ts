/**
 * Unit tests for the live HTTP task gateway.
 *
 * The suite validates success parsing plus shared-error mapping without
 * reaching a real backend.
 */
import { TaskGatewayError } from '../../ports/task-slice-gateway';
import { createHttpTaskGateway } from './http-task-gateway';

function createGatewayWithResponse(body: unknown, status = 200) {
  return createHttpTaskGateway(
    '/api/v1',
    vi
      .fn()
      .mockResolvedValue(
        new Response(JSON.stringify(body), { status }),
      ) as typeof fetch,
  );
}

describe('http task gateway', () => {
  const taskEnvelope = {
    data: {
      task: {
        created_at: '2026-04-13T00:00:00.000Z',
        id: 'task-1',
        origin: {
          issue_ref: {
            issue_number: 42,
            provider: 'github',
            repository: 'acme/widgets',
          },
          metadata: {
            assignees: [],
            labels: [],
            title: 'Stabilise the transport contract',
          },
          type: 'issue',
        },
        state: 'draft',
        updated_at: '2026-04-13T00:00:00.000Z',
      },
    },
    error: null,
    metadata: {
      request_id: 'req-123',
      timestamp: '2026-04-13T00:00:00.000Z',
      version: 'v1',
    },
    success: true,
  };

  it('parses successful task detail responses', async () => {
    const gateway = createGatewayWithResponse(taskEnvelope);

    await expect(gateway.getTask('task-1')).resolves.toMatchObject({
      id: 'task-1',
      state: 'draft',
    });
  });

  it('escapes task identifiers in task path segments', async () => {
    const fetchFn = vi
      .fn()
      .mockImplementation(() =>
        Promise.resolve(
          new Response(JSON.stringify(taskEnvelope), { status: 200 }),
        ),
      ) as typeof fetch;
    const gateway = createHttpTaskGateway('/api/v1', fetchFn);

    await gateway.getTask('task/with reserved?characters');
    await gateway.transitionTask('task/with reserved?characters', 'done');

    expect(fetchFn).toHaveBeenNthCalledWith(
      1,
      '/api/v1/tasks/task%2Fwith%20reserved%3Fcharacters',
      undefined,
    );
    expect(fetchFn).toHaveBeenNthCalledWith(
      2,
      '/api/v1/tasks/task%2Fwith%20reserved%3Fcharacters/state',
      expect.objectContaining({ method: 'PUT' }),
    );
  });

  it('rejects a 200 envelope when metadata is incomplete', async () => {
    const gateway = createGatewayWithResponse({
      ...taskEnvelope,
      metadata: {
        request_id: 'req-123',
        timestamp: '',
        version: 'v1',
      },
    });

    await expect(gateway.getTask('task-1')).rejects.toEqual(
      new TaskGatewayError(
        'unavailable',
        'The task API returned an invalid success response.',
      ),
    );
  });

  it('rejects a 200 envelope with an unsupported metadata version', async () => {
    const gateway = createGatewayWithResponse({
      ...taskEnvelope,
      metadata: {
        ...taskEnvelope.metadata,
        version: 'v2',
      },
    });

    await expect(gateway.getTask('task-1')).rejects.toEqual(
      new TaskGatewayError(
        'unavailable',
        'The task API returned an invalid success response.',
      ),
    );
  });

  it('maps shared not-found responses to the task gateway error', async () => {
    const gateway = createGatewayWithResponse(
      {
        code: 'not_found',
        details: { reason: 'task_not_found' },
        message: 'task task-9 was not found',
        traceId: 'trace-123',
      },
      404,
    );

    await expect(gateway.getTask('task-9')).rejects.toEqual(
      new TaskGatewayError('not_found', 'task task-9 was not found'),
    );
  });

  it('rejects a 200 envelope when the nested task violates the slice contract shape', async () => {
    const gateway = createGatewayWithResponse({
      data: {
        task: {
          created_at: '2026-04-13T00:00:00.000Z',
          id: 123,
          origin: {
            issue_ref: {
              issue_number: 42,
              provider: 'github',
              repository: 'acme/widgets',
            },
            metadata: {
              assignees: [],
              labels: [],
              title: 'Stabilise the transport contract',
            },
            type: 'issue',
          },
          state: 'draft',
          updated_at: '2026-04-13T00:00:00.000Z',
        },
      },
      error: null,
      metadata: {
        request_id: 'req-999',
        timestamp: '2026-04-13T00:00:00.000Z',
        version: 'v1',
      },
      success: true,
    });

    await expect(gateway.getTask('task-shape')).rejects.toEqual(
      new TaskGatewayError(
        'unavailable',
        'The task API returned an invalid task shape.',
      ),
    );
  });

  it('rejects a 200 envelope when the nested task has invalid timestamps', async () => {
    const gateway = createGatewayWithResponse({
      ...taskEnvelope,
      data: {
        task: {
          ...taskEnvelope.data.task,
          created_at: '2026-02-30T00:00:00.000Z',
        },
      },
    });

    await expect(gateway.getTask('task-shape')).rejects.toEqual(
      new TaskGatewayError(
        'unavailable',
        'The task API returned an invalid task shape.',
      ),
    );
  });

  it('maps invalid transition conflicts separately from validation failures', async () => {
    const gateway = createGatewayWithResponse(
      {
        code: 'conflict',
        details: {
          reason: 'invalid_task_transition',
          taskId: 'task-2',
        },
        message: 'invalid state transition for task task-2',
        traceId: 'trace-234',
      },
      409,
    );

    await expect(gateway.transitionTask('task-2', 'done')).rejects.toEqual(
      new TaskGatewayError(
        'conflict',
        'invalid state transition for task task-2',
      ),
    );
  });
});
