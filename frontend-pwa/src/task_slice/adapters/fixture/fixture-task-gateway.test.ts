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
