/**
 * Define the TanStack Router configuration for the frontend PWA.
 *
 * This module composes the root shell and task routes, then exports the router
 * factory consumed by the application bootstrap path.
 */
import {
  createRootRoute,
  createRoute,
  createRouter,
  Navigate,
} from '@tanstack/react-router';
import { TaskCreatePage } from '../routes/task-create-page';
import { TaskDetailPage } from '../routes/task-detail-page';
import { AppShell } from './app-shell';

const rootRoute = createRootRoute({
  component: AppShell,
});

const indexRoute = createRoute({
  getParentRoute: () => rootRoute,
  path: '/',
  component: () => <Navigate to="/tasks/new" />,
});

const tasksNewRoute = createRoute({
  getParentRoute: () => rootRoute,
  path: '/tasks/new',
  component: TaskCreatePage,
});

const taskDetailRoute = createRoute({
  getParentRoute: () => rootRoute,
  path: '/tasks/$taskId',
  component: TaskDetailPage,
});

const routeTree = rootRoute.addChildren([
  indexRoute,
  tasksNewRoute,
  taskDetailRoute,
]);

/**
 * Build the router instance from the assembled route tree.
 *
 * A factory is used so tests can create isolated router instances per case.
 */
export function createAppRouter() {
  return createRouter({ routeTree });
}
