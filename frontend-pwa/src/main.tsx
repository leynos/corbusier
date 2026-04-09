import { RouterProvider } from '@tanstack/react-router';
import ReactDOM from 'react-dom/client';

import { AppProviders } from './app/providers';
import { createAppRouter } from './app/router';
import { createFixtureTaskGateway } from './task_slice/adapters/fixture/fixture-task-gateway';
import './app/app.css';

const router = createAppRouter();
const gateway = createFixtureTaskGateway();
const rootElement = document.body.children.namedItem('root');

if (!(rootElement instanceof HTMLElement)) {
  throw new Error('Root element #root is missing.');
}

ReactDOM.createRoot(rootElement).render(
  <AppProviders gateway={gateway}>
    <RouterProvider router={router} />
  </AppProviders>,
);
