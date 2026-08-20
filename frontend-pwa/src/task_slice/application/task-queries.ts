/**
 * Provide React Query hooks for task creation and task detail reads.
 *
 * This module encapsulates query and mutation wiring so UI modules consume
 * stable task hooks instead of raw gateway calls.
 */
import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query';

import type { CreateTaskRequest } from '../domain/task';
import { useTaskGateway } from './task-gateway-context';

/**
 * Create a task through the gateway and seed the detail query cache. The
 * cached result renders immediately and avoids a refetch while fresh under
 * AppProviders' 30-second stale time, then revalidates after becoming stale.
 *
 * @example
 * `useCreateTaskMutation().mutate(request)` creates and caches the returned task.
 */
export function useCreateTaskMutation() {
  const gateway = useTaskGateway();
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: (request: CreateTaskRequest) => gateway.createTask(request),
    onSuccess(task) {
      queryClient.setQueryData(['task', task.id], task);
    },
  });
}

/**
 * Load task detail by id; retries are disabled so gateway errors (for
 * example not-found) surface immediately to the route.
 *
 * @example
 * `useTaskDetailQuery('task-1')` returns the query state for task `task-1`.
 */
export function useTaskDetailQuery(taskId: string) {
  const gateway = useTaskGateway();

  return useQuery({
    queryKey: ['task', taskId],
    queryFn: () => gateway.getTask(taskId),
    retry: false,
  });
}
