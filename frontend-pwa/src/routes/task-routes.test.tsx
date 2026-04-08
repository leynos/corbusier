import { screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { fixtureNotFoundTaskId } from '../task_slice/adapters/fixture/fixture-task-gateway';
import { renderApp } from '../test/test-utils';

describe('task routes', () => {
  it('creates a task and lands on detail', async () => {
    const user = userEvent.setup();
    await renderApp();

    await user.type(screen.getByLabelText('Repository'), 'acme/widgets');
    await user.type(screen.getByLabelText('Issue number'), '42');
    await user.type(screen.getByLabelText('Title'), 'Fix login flow');
    await user.click(screen.getByRole('button', { name: 'Create task' }));

    await waitFor(() => {
      expect(screen.getByText('Task detail')).toBeInTheDocument();
    });
    expect(screen.getByText('Fix login flow')).toBeInTheDocument();
    expect(
      screen.getByText('github/acme/widgets/#42', { selector: 'dd' }),
    ).toBeInTheDocument();
  });

  it('shows validation feedback for invalid task creation', async () => {
    const user = userEvent.setup();
    await renderApp();

    await user.click(screen.getByRole('button', { name: 'Create task' }));

    expect(
      screen.getByText('Use the repository format owner/repository.'),
    ).toBeInTheDocument();
    expect(
      screen.getByText('Issue number must be a positive integer.'),
    ).toBeInTheDocument();
    expect(screen.getByText('Title is required.')).toBeInTheDocument();
  });

  it('renders the not found task state', async () => {
    await renderApp({ initialPath: `/tasks/${fixtureNotFoundTaskId}` });

    await waitFor(() => {
      expect(screen.getByText('Task not found')).toBeInTheDocument();
    });
  });
});
