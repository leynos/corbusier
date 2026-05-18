/**
 * @file Unit tests for the `TaskNotFound` presentational component.
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

import { I18nProvider } from '../../i18n/runtime';
import { TaskNotFound } from './task-not-found';

async function renderComponent() {
  const rootRoute = createRootRoute({
    component: Outlet,
  });
  const route = createRoute({
    getParentRoute: () => rootRoute,
    path: '/missing',
    component: TaskNotFound,
  });
  const router = createRouter({
    history: createMemoryHistory({ initialEntries: ['/missing'] }),
    routeTree: rootRoute.addChildren([route]),
  });

  render(
    <I18nProvider>
      <RouterProvider router={router} />
    </I18nProvider>,
  );

  await act(async () => {
    await router.load();
  });
}

describe('TaskNotFound', () => {
  it('renders a not-found heading', async () => {
    await renderComponent();

    expect(
      screen.getByRole('heading', { name: 'Task not found' }),
    ).toBeInTheDocument();
  });

  it('renders explanatory body text', async () => {
    await renderComponent();

    expect(
      screen.getByText(
        'No fixture task matched this identifier. The live transport seam lands in roadmap item 4.4.2.',
      ),
    ).toBeInTheDocument();
  });

  it('renders a link back to the task-create route', async () => {
    await renderComponent();

    expect(
      screen.getByRole('link', { name: 'Return to task creation' }),
    ).toHaveAttribute('href', '/tasks/new');
  });

  it('matches the expected rendered output', async () => {
    await renderComponent();

    expect(
      screen.getByRole('heading', { name: 'Task not found' }).parentElement
        ?.innerHTML,
    ).toMatchInlineSnapshot(
      `"<h2 class="task-not-found__title">Task not found</h2><p class="task-not-found__body">No fixture task matched this identifier. The live transport seam lands in roadmap item 4.4.2.</p><a href="/tasks/new" class="btn btn-primary mt-6">Return to task creation</a>"`,
    );
  });
});
