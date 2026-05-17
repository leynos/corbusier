/**
 * Unit tests for the task gateway React context boundary.
 *
 * These tests exercise the provider/hook contract directly so missing
 * composition failures are caught before route-level tests obscure them.
 */
import { render, screen } from '@testing-library/react';

import type { TaskSliceGateway } from '../ports/task-slice-gateway';
import { TaskGatewayProvider, useTaskGateway } from './task-gateway-context';

function buildGateway(): TaskSliceGateway {
  return {
    createTask: async () => {
      throw new Error('not implemented');
    },
    getTask: async () => {
      throw new Error('not implemented');
    },
    transitionTask: async () => {
      throw new Error('not implemented');
    },
  };
}

function GatewayConsumer() {
  const gateway = useTaskGateway();

  return <output>{gateway ? 'gateway available' : 'gateway missing'}</output>;
}

describe('task gateway context', () => {
  it('provides the gateway to consumers', () => {
    render(
      <TaskGatewayProvider gateway={buildGateway()}>
        <GatewayConsumer />
      </TaskGatewayProvider>,
    );

    expect(screen.getByText('gateway available')).toBeInTheDocument();
  });

  it('throws when useTaskGateway is rendered without a provider', () => {
    expect(() => render(<GatewayConsumer />)).toThrow(
      'Task gateway provider is missing.',
    );
  });
});
