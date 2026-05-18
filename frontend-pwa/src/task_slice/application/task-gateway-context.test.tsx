/**
 * @file Unit tests for `TaskGatewayContext` and `useTaskGateway`.
 */
import { renderHook } from '@testing-library/react';

import { useTaskGateway } from './task-gateway-context';

describe('useTaskGateway', () => {
  it('throws when called outside a TaskGatewayProvider', () => {
    const spy = vi.spyOn(console, 'error').mockImplementation(() => undefined);

    expect(() => renderHook(() => useTaskGateway())).toThrow(
      'Task gateway provider is missing.',
    );

    spy.mockRestore();
  });
});
