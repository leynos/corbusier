/**
 * @file Not-found state component for the task detail route.
 *
 * Displayed when the task identified by the current route parameter does not
 * exist in the gateway. Renders a localized message and a navigational link
 * back to the task-create route. Contains no data-fetching or error-boundary
 * logic.
 */
import { Link } from '@tanstack/react-router';

import { useI18n } from '../../i18n/runtime';

/**
 * Renders the not-found state for the task detail route.
 *
 * Displays a localised heading, explanatory body text, and a navigation link
 * back to the task-create route at `/tasks/new`. Contains no data-fetching or
 * error-boundary logic; the parent route component owns gateway error mapping.
 */
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
