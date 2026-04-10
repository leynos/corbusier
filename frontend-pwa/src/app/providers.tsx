import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { type PropsWithChildren, useState } from 'react';

import { I18nProvider } from '../i18n/runtime';
import { TaskGatewayProvider } from '../task_slice/application/task-gateway-context';
import type { TaskSliceGateway } from '../task_slice/ports/task-slice-gateway';

export function AppProviders({
  children,
  gateway,
}: PropsWithChildren<{ gateway: TaskSliceGateway }>) {
  const [queryClient] = useState(
    () =>
      new QueryClient({
        defaultOptions: {
          queries: {
            staleTime: 30_000,
          },
        },
      }),
  );

  return (
    <I18nProvider>
      <QueryClientProvider client={queryClient}>
        <TaskGatewayProvider gateway={gateway}>{children}</TaskGatewayProvider>
      </QueryClientProvider>
    </I18nProvider>
  );
}
