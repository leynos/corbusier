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
  const state = formatTaskState(task.state);

  return (
    <section className="surface-panel rounded-[var(--corbusier-radius)] p-6">
      <div className="flex flex-col gap-4 md:flex-row md:items-start md:justify-between">
        <div className="space-y-2">
          <p className="text-sm uppercase tracking-[0.2em] text-[var(--corbusier-muted)]">
            {t('task.detail.title')}
          </p>
          <h2 className="text-3xl font-semibold">{task.origin.metadata.title}</h2>
          <p className="text-sm text-[var(--corbusier-muted)]">{formatIssueOrigin(task.origin)}</p>
        </div>
        <span className="status-pill" data-tone={state.tone}>
          {state.label}
        </span>
      </div>

      <dl className="mt-6 grid gap-4 md:grid-cols-2">
        <DetailItem label="Task ID" value={task.id} />
        <DetailItem label="Origin" value={formatIssueOrigin(task.origin)} />
        <DetailItem label="Created" value={formatTimestamp(task.created_at, locale)} />
        <DetailItem label="Updated" value={formatTimestamp(task.updated_at, locale)} />
        <DetailItem
          label="Branch reference"
          value={formatBranchRef(task.branch_ref) ?? t('task.refs.branch.empty')}
        />
        <DetailItem
          label="Pull request reference"
          value={formatPullRequestRef(task.pull_request_ref) ?? t('task.refs.pr.empty')}
        />
      </dl>

      {task.origin.metadata.description ? (
        <div className="mt-6 rounded-box bg-[var(--corbusier-accent-soft)]/60 p-4">
          <h3 className="font-semibold">Description</h3>
          <p className="mt-2 text-sm leading-6">{task.origin.metadata.description}</p>
        </div>
      ) : null}
    </section>
  );
}

function DetailItem({ label, value }: { label: string; value: string }) {
  return (
    <div className="rounded-box border border-[var(--corbusier-border)] bg-[var(--corbusier-surface-strong)]/30 p-4">
      <dt className="text-xs font-semibold uppercase tracking-[0.2em] text-[var(--corbusier-muted)]">
        {label}
      </dt>
      <dd className="mt-2 break-all text-sm leading-6">{value}</dd>
    </div>
  );
}
