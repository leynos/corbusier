import { Link } from '@tanstack/react-router';

import { useI18n } from '../../i18n/runtime';

export function TaskNotFound() {
  const { t } = useI18n();

  return (
    <section className="task-not-found surface-panel">
      <h2 className="task-not-found__title">{t('task.detail.notFound')}</h2>
      <p className="task-not-found__body">{t('task.detail.notFoundBody')}</p>
      <Link className="btn btn-primary mt-6" to="/tasks/new">
        {t('task.detail.notFoundAction')}
      </Link>
    </section>
  );
}
