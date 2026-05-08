/**
 * Unit tests for the fixture task gateway used by local development and tests.
 *
 * The suite validates `createFixtureTaskGateway` success paths and
 * `TaskGatewayError` surfaces for unavailable and not-found scenarios.
 */
import { TaskGatewayError } from '../../ports/task-slice-gateway';
import { createFixtureTaskGateway } from './fixture-task-gateway';

describe('fixture task gateway', () => {
  it('creates a draft task from issue metadata', async () => {
    const gateway = createFixtureTaskGateway();
    const task = await gateway.createTask({
      provider: 'github',
      repository: 'acme/widgets',
      issue_number: 77,
      title: 'Promote task shell',
    });

    expect(task.state).toBe('draft');
    expect(task.origin.issue_ref.issue_number).toBe(77);
    await expect(gateway.getTask(task.id)).resolves.toMatchObject({
      id: task.id,
      origin: {
        metadata: {
          title: 'Promote task shell',
        },
      },
    });
  });

  it('reports not found detail requests', async () => {
    const gateway = createFixtureTaskGateway();

    await expect(gateway.getTask('missing-task')).rejects.toEqual(
      new TaskGatewayError('not_found', 'Task missing-task was not found.'),
    );
  });

  it('transitions an existing task', async () => {
    const gateway = createFixtureTaskGateway();
    const task = await gateway.createTask({
      provider: 'github',
      repository: 'acme/widgets',
      issue_number: 33,
      title: 'Move task into progress',
    });

    await expect(
      gateway.transitionTask(task.id, 'in_progress'),
    ).resolves.toMatchObject({
      id: task.id,
      state: 'in_progress',
    });
    await expect(gateway.getTask(task.id)).resolves.toMatchObject({
      id: task.id,
      state: 'in_progress',
    });
  });

  it('rejects invalid state transitions before mutating a task', async () => {
    const gateway = createFixtureTaskGateway();
    const task = await gateway.createTask({
      provider: 'github',
      repository: 'acme/widgets',
      issue_number: 34,
      title: 'Reject invalid transition',
    });

    await expect(gateway.transitionTask(task.id, 'done')).rejects.toEqual(
      new TaskGatewayError(
        'conflict',
        `Invalid task transition for ${task.id}: cannot move from draft to done.`,
      ),
    );
    await expect(gateway.getTask(task.id)).resolves.toMatchObject({
      id: task.id,
      state: 'draft',
    });
  });

  it('simulates an unavailable submission path', async () => {
    const gateway = createFixtureTaskGateway();

    await expect(
      gateway.createTask({
        provider: 'github',
        repository: 'acme/widgets',
        issue_number: 9,
        title: '[fixture-error] fail',
      }),
    ).rejects.toEqual(
      new TaskGatewayError(
        'unavailable',
        'Fixture gateway rejected the task submission.',
      ),
    );
  });
});
