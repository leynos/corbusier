/**
 * @file Presentational card component for task detail display.
 *
 * Renders a `Task` value object's fields using the view-model formatters from
 * `task_slice/domain/task-view-model`. Locale-aware formatting (timestamps,
 * state labels, branch references, pull-request references) is delegated to
 * those formatters; this component contains no formatting logic of its own.
 */
import { useI18n } from '../../i18n/runtime';
import type { Task } from '../domain/task';
import {
  formatBranchRef,
  formatIssueOrigin,
  formatPullRequestRef,
  formatTaskState,
  formatTimestamp,
} from '../domain/task-view-model';

export function TaskDetailCard({ task }: { task: Task }) {
  const { locale, t } = useI18n();
  const state = formatTaskState(task.state, t);

  return (
    <section className="task-detail surface-panel">
      <div className="task-detail__header">
        <div className="task-detail__summary">
          <p className="task-detail__eyebrow">{t('task.detail.title')}</p>
          <h2 className="task-detail__title">{task.origin.metadata.title}</h2>
          <p className="task-detail__origin">
            {formatIssueOrigin(task.origin)}
          </p>
        </div>
        <span className="status-pill" data-tone={state.tone}>
          {state.label}
        </span>
      </div>

      <dl className="task-detail__meta-grid">
        <DetailItem label={t('task.detail.taskId')} value={task.id} />
        <DetailItem
          label={t('task.detail.origin')}
          value={formatIssueOrigin(task.origin)}
        />
        <DetailItem
          label={t('task.detail.created')}
          value={formatTimestamp(task.created_at, locale)}
        />
        <DetailItem
          label={t('task.detail.updated')}
          value={formatTimestamp(task.updated_at, locale)}
        />
        <DetailItem
          label={t('task.detail.branchRef')}
          value={
            formatBranchRef(task.branch_ref) ?? t('task.refs.branch.empty')
          }
        />
        <DetailItem
          label={t('task.detail.pullRequestRef')}
          value={
            formatPullRequestRef(task.pull_request_ref) ??
            t('task.refs.pr.empty')
          }
        />
      </dl>

      {task.origin.metadata.description ? (
        <div className="task-detail__description">
          <h3 className="task-detail__description-title">
            {t('task.detail.description')}
          </h3>
          <p className="task-detail__description-body">
            {task.origin.metadata.description}
          </p>
        </div>
      ) : null}
    </section>
  );
}

function DetailItem({ label, value }: { label: string; value: string }) {
  return (
    <div className="task-detail__meta-item">
      <dt className="task-detail__meta-label">{label}</dt>
      <dd className="task-detail__meta-value">{value}</dd>
    </div>
  );
}
