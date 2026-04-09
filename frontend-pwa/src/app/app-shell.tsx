import { Link, Outlet } from '@tanstack/react-router';

import { useI18n } from '../i18n/runtime';

export function AppShell() {
  const { t } = useI18n();

  return (
    <div className="app-shell">
      <header className="app-header sticky top-0 z-10">
        <div className="app-shell__header-inner">
          <div>
            <p className="app-shell__eyebrow">{t('app.subtitle')}</p>
            <h1 className="app-shell__title">{t('app.title')}</h1>
          </div>
          <nav className="app-shell__nav">
            <Link className="app-shell__nav-link" to="/tasks/new">
              {t('app.nav.newTask')}
            </Link>
            <Link
              className="app-shell__nav-link"
              to="/tasks/$taskId"
              params={{ taskId: '9f6adf0b-4908-47f5-a1fd-27d65f7d84bf' }}
            >
              {t('app.nav.seedDetail')}
            </Link>
          </nav>
        </div>
      </header>
      <main className="app-shell__main">
        <Outlet />
      </main>
    </div>
  );
}
