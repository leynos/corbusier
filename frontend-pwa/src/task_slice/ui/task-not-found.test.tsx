/**
 * Unit tests for the task not-found route state.
 *
 * The component owns only rendering and navigation affordances, so these tests
 * mount the smallest router needed for the TanStack `Link` contract.
 */
import {
  createMemoryHistory,
  createRootRoute,
  createRoute,
  createRouter,
  Outlet,
  RouterProvider,
} from '@tanstack/react-router';
import { act, render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';

import { I18nProvider } from '../../i18n/runtime';
import { TaskNotFound } from './task-not-found';

const createTaskHeading = 'Create task';

async function renderTaskNotFound() {
  const rootRoute = createRootRoute({
    component: Outlet,
  });
  const missingRoute = createRoute({
    getParentRoute: () => rootRoute,
    path: '/missing',
    component: TaskNotFound,
  });
  const createRouteEntry = createRoute({
    getParentRoute: () => rootRoute,
    path: '/tasks/new',
    component: () => <h2>{createTaskHeading}</h2>,
  });
  const router = createRouter({
    history: createMemoryHistory({ initialEntries: ['/missing'] }),
    routeTree: rootRoute.addChildren([missingRoute, createRouteEntry]),
  });

  const rendered = render(
    <I18nProvider>
      <RouterProvider router={router} />
    </I18nProvider>,
  );

  await act(async () => {
    await router.load();
  });

  return { router, ...rendered };
}

describe('TaskNotFound', () => {
  it('renders the localized not-found message and action', async () => {
    await renderTaskNotFound();

    expect(
      screen.getByRole('heading', { name: 'Task not found' }),
    ).toBeInTheDocument();
    expect(
      screen.getByText(
        'No fixture task matched this identifier. The live transport seam lands in roadmap item 4.4.2.',
      ),
    ).toBeInTheDocument();
    expect(
      screen.getByRole('link', { name: 'Return to task creation' }),
    ).toHaveAttribute('href', '/tasks/new');
  });

  it('navigates back to the task creation route', async () => {
    const user = userEvent.setup();
    const { router } = await renderTaskNotFound();

    await user.click(
      screen.getByRole('link', { name: 'Return to task creation' }),
    );

    expect(router.state.location.pathname).toBe('/tasks/new');
    expect(
      screen.getByRole('heading', { name: createTaskHeading }),
    ).toBeInTheDocument();
  });
});
