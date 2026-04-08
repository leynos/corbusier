import { Link, Outlet } from '@tanstack/react-router';

import { useI18n } from '../i18n/runtime';

export function AppShell() {
  const { t } = useI18n();

  return (
    <div className="app-shell">
      <header className="app-header sticky top-0 z-10">
        <div className="mx-auto flex max-w-6xl items-center justify-between px-6 py-4">
          <div>
            <p className="text-xs font-semibold uppercase tracking-[0.25em] text-[var(--corbusier-muted)]">
              {t('app.subtitle')}
            </p>
            <h1 className="text-2xl font-semibold">{t('app.title')}</h1>
          </div>
          <nav className="flex items-center gap-3">
            <Link className="btn btn-ghost btn-sm" to="/tasks/new">
              New task
            </Link>
            <Link
              className="btn btn-ghost btn-sm"
              to="/tasks/$taskId"
              params={{ taskId: '9f6adf0b-4908-47f5-a1fd-27d65f7d84bf' }}
            >
              Seed detail
            </Link>
          </nav>
        </div>
      </header>
      <main className="mx-auto max-w-6xl px-6 py-10">
        <Outlet />
      </main>
    </div>
  );
}
