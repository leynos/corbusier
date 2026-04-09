import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query';

import type { CreateTaskRequest } from '../domain/task';
import { useTaskGateway } from './task-gateway-context';

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

export function useTaskDetailQuery(taskId: string) {
  const gateway = useTaskGateway();

  return useQuery({
    queryKey: ['task', taskId],
    queryFn: () => gateway.getTask(taskId),
    retry: false,
  });
}
