/**
 * `@file` Shared test utilities for full-app render tests.
 *
 * Exports `renderApp`, which mounts the complete Corbusier PWA — including
 * routing, providers, and an injected `FixtureTaskGateway` — into a JSDOM
 * environment using Testing Library. Use this helper for integration tests
 * that exercise route transitions or cross-cutting provider behaviour; use
 * component-local render helpers for unit tests that do not need routing.
 */
import { createMemoryHistory, RouterProvider } from '@tanstack/react-router';
import { act, render } from '@testing-library/react';

import { AppProviders } from '../app/providers';
import { createAppRouter } from '../app/router';
import { createFixtureTaskGateway } from '../task_slice/adapters/fixture/fixture-task-gateway';
import type { Task } from '../task_slice/domain/task';

export async function renderApp(
  { initialPath = '/tasks/new', seedTasks } = {} as {
    initialPath?: string;
    seedTasks?: Task[];
  },
) {
  const history = createMemoryHistory({ initialEntries: [initialPath] });
  const router = createAppRouter();
  router.update({ history });
  const gateway = createFixtureTaskGateway(seedTasks);

  const rendered = render(
    <AppProviders gateway={gateway}>
      <RouterProvider router={router} />
    </AppProviders>,
  );

  await act(async () => {
    await router.load();
  });

  return rendered;
}
