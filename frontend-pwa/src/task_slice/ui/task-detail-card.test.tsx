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
  const { container } = render(
    <I18nProvider>
      <TaskDetailCard task={task} />
    </I18nProvider>,
  );
  return { container };
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

  it('matches the expected rendered structure for the base task', () => {
    const { container } = renderCard();

    expect(container.firstElementChild?.innerHTML).toMatchInlineSnapshot(
      `"<div class="task-detail__header"><div class="task-detail__summary"><p class="task-detail__eyebrow">Task detail</p><h2 class="task-detail__title">Fix login flow</h2><p class="task-detail__origin">github/acme/widgets/#42</p></div><span class="status-pill" data-tone="steady">Draft</span></div><dl class="task-detail__meta-grid"><div class="task-detail__meta-item"><dt class="task-detail__meta-label">Task ID</dt><dd class="task-detail__meta-value">aaaaaaaa-bbbb-cccc-dddd-eeeeeeeeeeee</dd></div><div class="task-detail__meta-item"><dt class="task-detail__meta-label">Origin</dt><dd class="task-detail__meta-value">github/acme/widgets/#42</dd></div><div class="task-detail__meta-item"><dt class="task-detail__meta-label">Created</dt><dd class="task-detail__meta-value">8 Apr 2026, 12:00</dd></div><div class="task-detail__meta-item"><dt class="task-detail__meta-label">Updated</dt><dd class="task-detail__meta-value">8 Apr 2026, 13:00</dd></div><div class="task-detail__meta-item"><dt class="task-detail__meta-label">Branch reference</dt><dd class="task-detail__meta-value">No branch linked yet. Live association lands in roadmap item 4.4.4.</dd></div><div class="task-detail__meta-item"><dt class="task-detail__meta-label">Pull request reference</dt><dd class="task-detail__meta-value">No pull request linked yet. Live association lands in roadmap item 4.4.4.</dd></div></dl><div class="task-detail__description"><h3 class="task-detail__description-title">Description</h3><p class="task-detail__description-body">A brief description.</p></div>"`,
    );
  });
});
