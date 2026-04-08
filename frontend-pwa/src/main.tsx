import { RouterProvider } from '@tanstack/react-router';
import ReactDOM from 'react-dom/client';

import { AppProviders } from './app/providers';
import { createFixtureTaskGateway } from './task_slice/adapters/fixture/fixture-task-gateway';
import { createAppRouter } from './app/router';
import './app/app.css';

const router = createAppRouter();
const gateway = createFixtureTaskGateway();

ReactDOM.createRoot(document.getElementById('root') as HTMLElement).render(
  <AppProviders gateway={gateway}>
    <RouterProvider router={router} />
  </AppProviders>,
);
