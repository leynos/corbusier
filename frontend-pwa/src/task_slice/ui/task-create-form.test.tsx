/**
 * @file Unit tests for the `TaskCreateForm` presentational component.
 */
import { fireEvent, render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import type { FormEvent } from 'react';

import { I18nProvider } from '../../i18n/runtime';
import type { TaskCreateDraft, TaskCreateErrors } from '../domain/task-form';
import { TaskCreateForm } from './task-create-form';

const emptyDraft: TaskCreateDraft = {
  provider: 'github',
  repository: '',
  issueNumber: '',
  title: '',
  description: '',
  labels: '',
  assignees: '',
  milestone: '',
};

const noErrors: TaskCreateErrors = {};

function renderForm(overrides?: {
  draft?: Partial<TaskCreateDraft>;
  errors?: TaskCreateErrors;
  isSubmitting?: boolean;
  submitError?: string;
  onChange?: (field: string, value: string) => void;
  onSubmit?: (event: FormEvent<HTMLFormElement>) => void;
}) {
  const onChange = overrides?.onChange ?? vi.fn();
  const onSubmit = overrides?.onSubmit ?? vi.fn();
  render(
    <I18nProvider>
      <TaskCreateForm
        draft={{ ...emptyDraft, ...overrides?.draft }}
        errors={overrides?.errors ?? noErrors}
        isSubmitting={overrides?.isSubmitting ?? false}
        submitError={overrides?.submitError}
        onChange={onChange}
        onSubmit={onSubmit}
      />
    </I18nProvider>,
  );
  return { onChange, onSubmit };
}

describe('TaskCreateForm', () => {
  it('renders all form fields', () => {
    renderForm();

    expect(screen.getByLabelText('Provider')).toBeInTheDocument();
    expect(screen.getByLabelText('Repository')).toBeInTheDocument();
    expect(screen.getByLabelText('Issue number')).toBeInTheDocument();
    expect(screen.getByLabelText('Title')).toBeInTheDocument();
    expect(screen.getByLabelText('Description')).toBeInTheDocument();
    expect(screen.getByLabelText('Labels')).toBeInTheDocument();
    expect(screen.getByLabelText('Assignees')).toBeInTheDocument();
    expect(screen.getByLabelText('Milestone')).toBeInTheDocument();
  });

  it('renders the submit button with the create label', () => {
    renderForm();

    expect(
      screen.getByRole('button', { name: 'Create task' }),
    ).toBeInTheDocument();
  });

  it('disables the submit button while submitting', () => {
    renderForm({ isSubmitting: true });

    expect(
      screen.getByRole('button', { name: 'Creating task…' }),
    ).toBeDisabled();
  });

  it('displays a submit-error alert when submitError is set', () => {
    renderForm({ submitError: 'Something went wrong.' });

    expect(screen.getByRole('alert')).toHaveTextContent(
      'Something went wrong.',
    );
  });

  it('displays a field validation error when provided', () => {
    renderForm({ errors: { repository: 'Use the format owner/repository.' } });

    expect(
      screen.getByText('Use the format owner/repository.'),
    ).toBeInTheDocument();
  });

  it('calls onChange when a field value changes', async () => {
    const user = userEvent.setup();
    const { onChange } = renderForm();

    await user.type(screen.getByLabelText('Repository'), 'a');

    expect(onChange).toHaveBeenCalledWith('repository', expect.any(String));
  });

  it('calls onSubmit when the form is submitted', async () => {
    const { onSubmit } = renderForm();

    const submitButton = screen.getByRole('button', { name: 'Create task' });
    fireEvent.submit(submitButton.closest('form') as HTMLFormElement);

    expect(onSubmit).toHaveBeenCalledTimes(1);
  });
});
