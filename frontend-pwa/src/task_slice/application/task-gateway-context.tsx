/**
 * `@file` React context and hook for the `TaskSliceGateway` port.
 *
 * Exports `TaskGatewayProvider`, which places a `TaskSliceGateway`
 * implementation into React context, and `useTaskGateway`, which retrieves it.
 * Components and hooks in `task_slice/application/` consume the gateway
 * exclusively through `useTaskGateway` to preserve the hexagonal boundary and
 * keep infrastructure adapters out of the component tree.
 */
import { createContext, type PropsWithChildren, useContext } from 'react';

import type { TaskSliceGateway } from '../ports/task-slice-gateway';

const TaskGatewayContext = createContext<TaskSliceGateway | null>(null);

export function TaskGatewayProvider({
  children,
  gateway,
}: PropsWithChildren<{ gateway: TaskSliceGateway }>) {
  return (
    <TaskGatewayContext.Provider value={gateway}>
      {children}
    </TaskGatewayContext.Provider>
  );
}

export function useTaskGateway() {
  const gateway = useContext(TaskGatewayContext);
  if (!gateway) {
    throw new Error('Task gateway provider is missing.');
  }

  return gateway;
}
