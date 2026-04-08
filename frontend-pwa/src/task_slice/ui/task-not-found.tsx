import { Link } from '@tanstack/react-router';

import { useI18n } from '../../i18n/runtime';

export function TaskNotFound() {
  const { t } = useI18n();

  return (
    <section className="surface-panel rounded-[var(--corbusier-radius)] p-6">
      <h2 className="text-2xl font-semibold">{t('task.detail.notFound')}</h2>
      <p className="mt-3 max-w-2xl text-sm leading-6 text-[var(--corbusier-muted)]">
        {t('task.detail.notFoundBody')}
      </p>
      <Link className="btn btn-primary mt-6" to="/tasks/new">
        Return to task creation
      </Link>
    </section>
  );
}
