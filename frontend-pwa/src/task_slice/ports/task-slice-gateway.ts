/**
 * Task-slice port definitions shared by adapters and consumers.
 *
 * This module defines the `TaskSliceGateway` contract plus the error kinds used
 * to communicate task-related adapter failures across the slice boundary.
 */
import type { CreateTaskRequest, Task, TaskState } from '../domain/task';

export type TaskGatewayErrorKind =
  | 'not_found'
  | 'validation'
  | 'conflict'
  | 'unauthorized'
  | 'unavailable';

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
  transitionTask(taskId: string, targetState: TaskState): Promise<Task>;
}
