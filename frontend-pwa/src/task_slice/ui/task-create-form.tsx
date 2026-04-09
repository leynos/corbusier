import type { FormEvent } from 'react';

import { useI18n } from '../../i18n/runtime';
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
  const { t } = useI18n();

  return (
    <form className="task-create-form" onSubmit={onSubmit}>
      {submitError ? (
        <div className="alert alert-error" role="alert">
          <span>{submitError}</span>
        </div>
      ) : null}
      <div className="task-create-form__row">
        <label className="form-control w-full">
          <span className="label-text mb-2 font-semibold">
            {t('task.form.provider')}
          </span>
          <select
            className="select select-bordered"
            name="provider"
            value={draft.provider}
            onChange={(event) => onChange('provider', event.target.value)}
          >
            <option value="github">{t('task.form.provider.github')}</option>
            <option value="gitlab">{t('task.form.provider.gitlab')}</option>
          </select>
          {errors.provider ? (
            <span className="label-text-alt text-error">{errors.provider}</span>
          ) : null}
        </label>
        <Field
          error={errors.repository}
          label={t('task.form.repository')}
          name="repository"
          placeholder={t('task.form.repository.placeholder')}
          value={draft.repository}
          onChange={onChange}
        />
      </div>
      <div className="task-create-form__row">
        <Field
          error={errors.issueNumber}
          label={t('task.form.issueNumber')}
          name="issueNumber"
          placeholder={t('task.form.issueNumber.placeholder')}
          value={draft.issueNumber}
          onChange={onChange}
        />
        <Field
          error={errors.title}
          label={t('task.form.title')}
          name="title"
          placeholder={t('task.form.title.placeholder')}
          value={draft.title}
          onChange={onChange}
        />
      </div>
      <label className="form-control">
        <span className="label-text mb-2 font-semibold">
          {t('task.form.description')}
        </span>
        <textarea
          className="textarea textarea-bordered min-h-28"
          name="description"
          placeholder={t('task.form.description.placeholder')}
          value={draft.description}
          onChange={(event) => onChange('description', event.target.value)}
        />
      </label>
      <div className="task-create-form__row">
        <Field
          label={t('task.form.labels')}
          name="labels"
          placeholder={t('task.form.labels.placeholder')}
          value={draft.labels}
          onChange={onChange}
        />
        <Field
          label={t('task.form.assignees')}
          name="assignees"
          placeholder={t('task.form.assignees.placeholder')}
          value={draft.assignees}
          onChange={onChange}
        />
      </div>
      <Field
        label={t('task.form.milestone')}
        name="milestone"
        placeholder={t('task.form.milestone.placeholder')}
        value={draft.milestone}
        onChange={onChange}
      />
      <button className="btn btn-primary" type="submit" disabled={isSubmitting}>
        {isSubmitting ? t('task.form.submitting') : t('task.form.submit')}
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

function Field({
  error,
  label,
  name,
  placeholder,
  value,
  onChange,
}: FieldProps) {
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
      {error ? (
        <span className="label-text-alt text-error">{error}</span>
      ) : null}
    </label>
  );
}
