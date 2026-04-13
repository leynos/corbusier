/**
 * Bootstrap the repository-owned frontend PWA.
 *
 * This entrypoint wires the router, providers, and fixture task gateway before
 * mounting the React tree into the root DOM element for local and CI runs.
 */
import { RouterProvider } from '@tanstack/react-router';
import ReactDOM from 'react-dom/client';

import { AppProviders } from './app/providers';
import { createAppRouter } from './app/router';
import { createFixtureTaskGateway } from './task_slice/adapters/fixture/fixture-task-gateway';
import { createHttpTaskGateway } from './task_slice/adapters/http/http-task-gateway';
import './app/app.css';

const router = createAppRouter();
const gateway = createTaskGateway();
const rootElement = document.body.children.namedItem('root');

if (!(rootElement instanceof HTMLElement)) {
  throw new Error('Root element #root is missing.');
}

ReactDOM.createRoot(rootElement).render(
  <AppProviders gateway={gateway}>
    <RouterProvider router={router} />
  </AppProviders>,
);

function createTaskGateway() {
  if (import.meta.env.VITE_TASK_GATEWAY_MODE === 'http') {
    const baseUrl = import.meta.env.VITE_CORBUSIER_API_BASE_URL ?? '/api/v1';
    return createHttpTaskGateway(baseUrl);
  }

  return createFixtureTaskGateway();
}
