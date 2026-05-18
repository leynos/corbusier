/**
 * @file Unit tests for the `TaskDetailCard` presentational component.
 */
import { render, screen } from '@testing-library/react';

import { I18nProvider } from '../../i18n/runtime';
import type { Task } from '../domain/task';
import { TaskDetailCard } from './task-detail-card';

const baseTask: Task = {
  id: 'aaaaaaaa-bbbb-cccc-dddd-eeeeeeeeeeee',
  origin: {
    type: 'issue',
    issue_ref: {
      provider: 'github',
      repository: 'acme/widgets',
      issue_number: 42,
    },
    metadata: {
      title: 'Fix login flow',
      description: 'A brief description.',
      labels: [],
      assignees: [],
    },
  },
  state: 'draft',
  created_at: '2026-04-08T12:00:00.000Z',
  updated_at: '2026-04-08T13:00:00.000Z',
};

function renderCard(task: Task = baseTask) {
  render(
    <I18nProvider>
      <TaskDetailCard task={task} />
    </I18nProvider>,
  );
}

describe('TaskDetailCard', () => {
  it('renders the task title as a heading', () => {
    renderCard();

    expect(
      screen.getByRole('heading', { name: 'Fix login flow' }),
    ).toBeInTheDocument();
  });

  it('renders the task ID', () => {
    renderCard();

    expect(
      screen.getByText('aaaaaaaa-bbbb-cccc-dddd-eeeeeeeeeeee'),
    ).toBeInTheDocument();
  });

  it('renders the description when present', () => {
    renderCard();

    expect(screen.getByText('A brief description.')).toBeInTheDocument();
  });

  it('does not render the description section when absent', () => {
    renderCard({
      ...baseTask,
      origin: {
        ...baseTask.origin,
        metadata: { ...baseTask.origin.metadata, description: undefined },
      },
    });

    expect(screen.queryByText('Description')).not.toBeInTheDocument();
  });

  it('renders all expected meta labels', () => {
    renderCard();

    for (const label of ['Task ID', 'Origin', 'Created', 'Updated']) {
      expect(screen.getByText(label)).toBeInTheDocument();
    }
  });
});
