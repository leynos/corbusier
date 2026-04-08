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

export function createAppRouter() {
  return createRouter({ routeTree });
}
