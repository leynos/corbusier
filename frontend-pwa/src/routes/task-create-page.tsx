import { useNavigate } from '@tanstack/react-router';
import { type FormEvent, useState } from 'react';

import { useI18n } from '../i18n/runtime';
import { useCreateTaskMutation } from '../task_slice/application/task-queries';
import {
  initialTaskCreateDraft,
  type TaskCreateDraft,
  type TaskCreateErrors,
  type TaskCreateField,
  toCreateTaskRequest,
  validateTaskCreateDraft,
} from '../task_slice/domain/task-form';
import { TaskCreateForm } from '../task_slice/ui/task-create-form';

export function TaskCreatePage() {
  const { t } = useI18n();
  const navigate = useNavigate();
  const mutation = useCreateTaskMutation();
  const [draft, setDraft] = useState<TaskCreateDraft>(initialTaskCreateDraft);
  const [errors, setErrors] = useState<TaskCreateErrors>({});

  function handleChange(field: TaskCreateField, value: string) {
    setDraft((current) => ({ ...current, [field]: value }));
    setErrors((current) => ({ ...current, [field]: undefined }));
  }

  async function handleSubmit(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();

    const nextErrors = validateTaskCreateDraft(draft);
    setErrors(nextErrors);
    if (Object.keys(nextErrors).length > 0) {
      return;
    }

    try {
      const task = await mutation.mutateAsync(toCreateTaskRequest(draft));
      await navigate({
        to: '/tasks/$taskId',
        params: { taskId: task.id },
      });
    } catch {
      // mutation.error will be set by React Query; allow the component to re-render
    }
  }

  return (
    <div className="task-create__layout">
      <section className="task-create__hero hero-panel">
        <p className="task-create__eyebrow">{t('task.intake.kicker')}</p>
        <h2 className="task-create__title">{t('task.create.title')}</h2>
        <p className="task-create__body">{t('task.create.description')}</p>
      </section>
      <aside className="task-create__form-panel surface-panel">
        <TaskCreateForm
          draft={draft}
          errors={errors}
          isSubmitting={mutation.isPending}
          submitError={mutation.error ? t('task.form.errorBanner') : undefined}
          onChange={handleChange}
          onSubmit={handleSubmit}
        />
      </aside>
    </div>
  );
}
