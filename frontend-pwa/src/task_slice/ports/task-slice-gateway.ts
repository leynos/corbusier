import type { CreateTaskRequest, Task } from '../domain/task';

export type TaskGatewayErrorKind = 'not_found' | 'validation' | 'unavailable';

export class TaskGatewayError extends Error {
  constructor(
    readonly kind: TaskGatewayErrorKind,
    message: string,
  ) {
    super(message);
  }
}

export interface TaskSliceGateway {
  createTask(request: CreateTaskRequest): Promise<Task>;
  getTask(taskId: string): Promise<Task>;
}
