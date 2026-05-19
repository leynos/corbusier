/**
 * @file Fixture task gateway for local development and tests.
 *
 * Exports {@link createFixtureTaskGateway} and {@link fixtureNotFoundTaskId}.
 * Use this module in place of the live HTTP adapter to exercise the PWA task
 * slice without a running backend.
 */
import type { CreateTaskRequest, Task, TaskState } from '../../domain/task';
import {
  TaskGatewayError,
  type TaskSliceGateway,
} from '../../ports/task-slice-gateway';

const notFoundTaskId = '11111111-1111-1111-1111-111111111111';

const allowedTransitions: Readonly<Record<TaskState, readonly TaskState[]>> = {
  abandoned: [],
  done: [],
  draft: ['in_progress', 'in_review', 'abandoned'],
  in_progress: ['in_review', 'paused', 'done', 'abandoned'],
  in_review: ['in_progress', 'done', 'abandoned'],
  paused: ['in_progress', 'abandoned'],
};

/**
 * Creates a deterministic in-memory `TaskSliceGateway` for tests and local
 * development.
 *
 * All gateway operations are serialized through an internal promise queue so
 * that concurrent callers observe consistent task state without races.
 *
 * @param seedTasks - Initial task records to populate the store; defaults to
 *   a single seed task. Pass `[]` for an empty store.
 */
export function createFixtureTaskGateway(
  seedTasks: Task[] = [buildSeedTask()],
): TaskSliceGateway {
  const tasks = new Map(seedTasks.map((task) => [task.id, task]));
  let operationQueue = Promise.resolve();

  function enqueue<T>(operation: () => Promise<T>): Promise<T> {
    const nextOperation = operationQueue.then(operation, operation);
    operationQueue = nextOperation.then(
      () => undefined,
      () => undefined,
    );

    return nextOperation;
  }

  return {
    async createTask(request) {
      return enqueue(async () => {
        await delay();
        if (request.title.toLowerCase().includes('[fixture-error]')) {
          throw new TaskGatewayError(
            'unavailable',
            'Fixture gateway rejected the task submission.',
          );
        }

        const task = buildTaskFromRequest(request);
        tasks.set(task.id, task);
        return task;
      });
    },
    async getTask(taskId) {
      return enqueue(async () => {
        await delay();
        const task = tasks.get(taskId);
        if (taskId === notFoundTaskId || !task) {
          throw new TaskGatewayError(
            'not_found',
            `Task ${taskId} was not found.`,
          );
        }

        return task;
      });
    },
    async transitionTask(taskId, targetState) {
      return enqueue(async () => {
        await delay();
        const existingTask = tasks.get(taskId);
        if (taskId === notFoundTaskId || !existingTask) {
          throw new TaskGatewayError(
            'not_found',
            `Task ${taskId} was not found.`,
          );
        }

        assertTransitionAllowed(taskId, existingTask.state, targetState);
        const updatedTask = applyTransition(existingTask, targetState);
        tasks.set(taskId, updatedTask);
        return updatedTask;
      });
    },
  };
}

/**
 * Throws a `conflict` `TaskGatewayError` if `targetState` is not a permitted
 * successor of `currentState` according to `allowedTransitions`.
 */
function assertTransitionAllowed(
  taskId: string,
  currentState: TaskState,
  targetState: TaskState,
): void {
  if (!allowedTransitions[currentState].includes(targetState)) {
    throw new TaskGatewayError(
      'conflict',
      `Invalid task transition for ${taskId}: cannot move from ${currentState} to ${targetState}.`,
    );
  }
}

/**
 * Constructs a new `Task` in the `draft` state from a `CreateTaskRequest`.
 *
 * Assigns a fresh UUID and sets both timestamps to the current instant.
 */
function buildTaskFromRequest(request: CreateTaskRequest): Task {
  const timestamp = new Date().toISOString();

  return {
    id: crypto.randomUUID(),
    origin: {
      type: 'issue',
      issue_ref: {
        provider: request.provider,
        repository: request.repository,
        issue_number: request.issue_number,
      },
      metadata: {
        title: request.title,
        description: request.description,
        labels: request.labels ?? [],
        assignees: request.assignees ?? [],
        milestone: request.milestone,
      },
    },
    state: 'draft',
    created_at: timestamp,
    updated_at: timestamp,
  };
}

/**
 * Returns the default seed `Task` used when no explicit seed array is
 * supplied to `createFixtureTaskGateway`.
 */
function buildSeedTask(): Task {
  const timestamp = '2026-04-08T12:00:00.000Z';

  return {
    id: '9f6adf0b-4908-47f5-a1fd-27d65f7d84bf',
    origin: {
      type: 'issue',
      issue_ref: {
        provider: 'github',
        repository: 'acme/widgets',
        issue_number: 42,
      },
      metadata: {
        title: 'Stabilise fixture-backed task slice',
        description:
          'Carry the narrow task detail contract into the PWA shell.',
        labels: ['frontend', 'roadmap-4.4.1'],
        assignees: ['alice'],
        milestone: 'sprint-12',
      },
    },
    branch_ref: {
      provider: 'github',
      repository: 'acme/widgets',
      branch_name: 'feature/task-shell',
    },
    pull_request_ref: {
      provider: 'github',
      repository: 'acme/widgets',
      pull_request_number: 108,
    },
    state: 'in_review',
    created_at: timestamp,
    updated_at: timestamp,
  };
}

/**
 * Returns a copy of `task` with `state` set to `targetState` and
 * `updated_at` refreshed to the current instant.
 */
function applyTransition(task: Task, targetState: TaskState): Task {
  return {
    ...task,
    state: targetState,
    updated_at: new Date().toISOString(),
  };
}

/**
 * Resolves after a short fixed delay to simulate network latency in tests
 * and local development.
 */
function delay() {
  return new Promise((resolve) => window.setTimeout(resolve, 15));
}

/**
 * Stable fixture task identifier that always resolves to a `not_found`
 * gateway error.
 */
export const fixtureNotFoundTaskId = notFoundTaskId;
