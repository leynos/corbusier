import { useParams } from '@tanstack/react-router';

import { useI18n } from '../i18n/runtime';
import { useTaskDetailQuery } from '../task_slice/application/task-queries';
import { TaskGatewayError } from '../task_slice/ports/task-slice-gateway';
import { TaskDetailCard } from '../task_slice/ui/task-detail-card';
import { TaskNotFound } from '../task_slice/ui/task-not-found';

export function TaskDetailPage() {
  const { t } = useI18n();
  const { taskId } = useParams({ from: '/tasks/$taskId' });
  const query = useTaskDetailQuery(taskId);

  if (query.isPending) {
    return (
      <div
        className="loading loading-spinner loading-lg"
        role="status"
        aria-label={t('task.detail.loading')}
      />
    );
  }

  if (
    query.error instanceof TaskGatewayError &&
    query.error.kind === 'not_found'
  ) {
    return <TaskNotFound />;
  }

  if (query.error) {
    return (
      <div className="alert alert-error" role="alert">
        <span>{query.error.message}</span>
      </div>
    );
  }

  return <TaskDetailCard task={query.data} />;
}
