/**
 * @file React context and hook for the `TaskSliceGateway` port.
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

/**
 * Injects a `TaskSliceGateway` implementation into the React context tree.
 *
 * Wrap application entry points and route-level test trees with this provider
 * so that child components can retrieve the gateway via {@link useTaskGateway}.
 */
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

/**
 * Retrieves the nearest `TaskSliceGateway` from context.
 *
 * Throws an `Error` with the message `"Task gateway provider is missing."`
 * when called outside a `TaskGatewayProvider` tree.
 */
export function useTaskGateway() {
  const gateway = useContext(TaskGatewayContext);
  if (!gateway) {
    throw new Error('Task gateway provider is missing.');
  }

  return gateway;
}
