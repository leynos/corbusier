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
 * Create a task through the gateway and seed the detail query cache with
 * the result so the next detail view avoids a redundant fetch.
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
 */
export function useTaskDetailQuery(taskId: string) {
  const gateway = useTaskGateway();

  return useQuery({
    queryKey: ['task', taskId],
    queryFn: () => gateway.getTask(taskId),
    retry: false,
  });
}
