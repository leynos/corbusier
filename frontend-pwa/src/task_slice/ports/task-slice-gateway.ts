/**
 * Task-slice port definitions shared by adapters and consumers.
 *
 * This module defines the `TaskSliceGateway` contract plus the error kinds used
 * to communicate task-related adapter failures across the slice boundary.
 */
import type { CreateTaskRequest, Task, TaskState } from '../domain/task';

/** Adapter-agnostic classification of task gateway failures. */
export type TaskGatewayErrorKind =
  | 'not_found'
  | 'validation'
  | 'conflict'
  | 'unauthorized'
  | 'unavailable';

/**
 * Error raised by a `TaskSliceGateway` adapter, tagged with a kind so
 * callers can branch on failure category without inspecting messages.
 */
export class TaskGatewayError extends Error {
  constructor(
    /** Category of the underlying adapter failure. */
    readonly kind: TaskGatewayErrorKind,
    message: string,
  ) {
    super(message);
  }
}

/** Port through which the task slice reaches its backing task store. */
export interface TaskSliceGateway {
  /** Create a task from an issue reference. */
  createTask(request: CreateTaskRequest): Promise<Task>;
  /** Fetch a task by id. */
  getTask(taskId: string): Promise<Task>;
  /** Move a task to `targetState`. */
  transitionTask(taskId: string, targetState: TaskState): Promise<Task>;
}
