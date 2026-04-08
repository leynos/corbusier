import type { CreateTaskRequest, Task } from '../../domain/task';
import {
  TaskGatewayError,
  type TaskSliceGateway,
} from '../../ports/task-slice-gateway';

export function createHttpTaskGateway(_baseUrl: string): TaskSliceGateway {
  return {
    async createTask(_request: CreateTaskRequest): Promise<Task> {
      throw new TaskGatewayError(
        'unavailable',
        'The live HTTP adapter is intentionally deferred to roadmap item 4.4.2.',
      );
    },
    async getTask(_taskId: string): Promise<Task> {
      throw new TaskGatewayError(
        'unavailable',
        'The live HTTP adapter is intentionally deferred to roadmap item 4.4.2.',
      );
    },
  };
}
