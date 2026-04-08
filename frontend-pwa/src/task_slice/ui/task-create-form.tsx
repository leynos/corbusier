import type { FormEvent } from 'react';

import type {
  TaskCreateDraft,
  TaskCreateErrors,
  TaskCreateField,
} from '../domain/task-form';

interface TaskCreateFormProps {
  draft: TaskCreateDraft;
  errors: TaskCreateErrors;
  isSubmitting: boolean;
  submitError?: string;
  onChange(field: TaskCreateField, value: string): void;
  onSubmit(event: FormEvent<HTMLFormElement>): void;
}

export function TaskCreateForm({
  draft,
  errors,
  isSubmitting,
  submitError,
  onChange,
  onSubmit,
}: TaskCreateFormProps) {
  return (
    <form className="space-y-4" onSubmit={onSubmit}>
      {submitError ? (
        <div className="alert alert-error" role="alert">
          <span>{submitError}</span>
        </div>
      ) : null}
      <div className="grid gap-4 md:grid-cols-2">
        <label className="form-control w-full">
          <span className="label-text mb-2 font-semibold">Provider</span>
          <select
            className="select select-bordered"
            name="provider"
            value={draft.provider}
            onChange={(event) => onChange('provider', event.target.value)}
          >
            <option value="github">GitHub</option>
            <option value="gitlab">GitLab</option>
          </select>
          {errors.provider ? <span className="label-text-alt text-error">{errors.provider}</span> : null}
        </label>
        <Field
          error={errors.repository}
          label="Repository"
          name="repository"
          placeholder="owner/repository"
          value={draft.repository}
          onChange={onChange}
        />
      </div>
      <div className="grid gap-4 md:grid-cols-2">
        <Field
          error={errors.issueNumber}
          label="Issue number"
          name="issueNumber"
          placeholder="42"
          value={draft.issueNumber}
          onChange={onChange}
        />
        <Field
          error={errors.title}
          label="Title"
          name="title"
          placeholder="Fix login flow"
          value={draft.title}
          onChange={onChange}
        />
      </div>
      <label className="form-control">
        <span className="label-text mb-2 font-semibold">Description</span>
        <textarea
          className="textarea textarea-bordered min-h-28"
          name="description"
          placeholder="Optional issue synopsis"
          value={draft.description}
          onChange={(event) => onChange('description', event.target.value)}
        />
      </label>
      <div className="grid gap-4 md:grid-cols-2">
        <Field
          label="Labels"
          name="labels"
          placeholder="bug, p1"
          value={draft.labels}
          onChange={onChange}
        />
        <Field
          label="Assignees"
          name="assignees"
          placeholder="alice, bob"
          value={draft.assignees}
          onChange={onChange}
        />
      </div>
      <Field
        label="Milestone"
        name="milestone"
        placeholder="sprint-12"
        value={draft.milestone}
        onChange={onChange}
      />
      <button className="btn btn-primary" type="submit" disabled={isSubmitting}>
        {isSubmitting ? 'Creating task…' : 'Create task'}
      </button>
    </form>
  );
}

interface FieldProps {
  error?: string;
  label: string;
  name: TaskCreateField;
  placeholder: string;
  value: string;
  onChange(field: TaskCreateField, value: string): void;
}

function Field({ error, label, name, placeholder, value, onChange }: FieldProps) {
  return (
    <label className="form-control w-full">
      <span className="label-text mb-2 font-semibold">{label}</span>
      <input
        className="input input-bordered w-full"
        name={name}
        placeholder={placeholder}
        value={value}
        onChange={(event) => onChange(name, event.target.value)}
      />
      {error ? <span className="label-text-alt text-error">{error}</span> : null}
    </label>
  );
}
