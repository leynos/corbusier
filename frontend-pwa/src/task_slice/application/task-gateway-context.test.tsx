/**
 * @file Unit tests for `TaskGatewayContext` and `useTaskGateway`.
 */
import { renderHook } from '@testing-library/react';
import type { PropsWithChildren } from 'react';

import type { TaskSliceGateway } from '../ports/task-slice-gateway';
import { TaskGatewayProvider, useTaskGateway } from './task-gateway-context';

describe('useTaskGateway', () => {
  it('throws when called outside a TaskGatewayProvider', () => {
    const spy = vi.spyOn(console, 'error').mockImplementation(() => undefined);

    expect(() => renderHook(() => useTaskGateway())).toThrow(
      'Task gateway provider is missing.',
    );

    spy.mockRestore();
  });

  it('returns the gateway injected by TaskGatewayProvider', () => {
    const mockGateway = {
      createTask: vi.fn(),
      getTask: vi.fn(),
      transitionTask: vi.fn(),
    } satisfies TaskSliceGateway;

    function Wrapper({ children }: PropsWithChildren) {
      return (
        <TaskGatewayProvider gateway={mockGateway}>
          {children}
        </TaskGatewayProvider>
      );
    }

    const { result } = renderHook(() => useTaskGateway(), { wrapper: Wrapper });

    expect(result.current).toBe(mockGateway);
  });
});
