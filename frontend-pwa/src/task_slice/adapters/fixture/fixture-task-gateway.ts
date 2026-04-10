/**
 * Implement a deterministic in-memory task gateway for tests and local runs.
 *
 * This module exports `createFixtureTaskGateway` plus fixture ids used to
 * simulate seeded and not-found task states without live backend traffic.
 */
import type { CreateTaskRequest, Task } from '../../domain/task';
import {
  TaskGatewayError,
  type TaskSliceGateway,
} from '../../ports/task-slice-gateway';

const notFoundTaskId = '11111111-1111-1111-1111-111111111111';

export function createFixtureTaskGateway(
  seedTasks: Task[] = [buildSeedTask()],
): TaskSliceGateway {
  const tasks = new Map(seedTasks.map((task) => [task.id, task]));

  return {
    async createTask(request) {
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
    },
    async getTask(taskId) {
      await delay();
      if (taskId === notFoundTaskId || !tasks.has(taskId)) {
        throw new TaskGatewayError(
          'not_found',
          `Task ${taskId} was not found.`,
        );
      }

      return tasks.get(taskId) as Task;
    },
  };
}

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

function delay() {
  return new Promise((resolve) => window.setTimeout(resolve, 15));
}

export const fixtureNotFoundTaskId = notFoundTaskId;
