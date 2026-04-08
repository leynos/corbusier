import { createContext, useContext, type PropsWithChildren } from 'react';

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
