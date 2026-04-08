import { useNavigate } from '@tanstack/react-router';
import { type FormEvent, useState } from 'react';

import { useI18n } from '../i18n/runtime';
import { useCreateTaskMutation } from '../task_slice/application/task-queries';
import {
  initialTaskCreateDraft,
  type TaskCreateDraft,
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
  const [errors, setErrors] = useState({});

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

    const task = await mutation.mutateAsync(toCreateTaskRequest(draft));
    await navigate({
      to: '/tasks/$taskId',
      params: { taskId: task.id },
    });
  }

  return (
    <div className="grid gap-6 lg:grid-cols-[1.1fr_0.9fr]">
      <section className="hero-panel rounded-[var(--corbusier-radius)] p-8">
        <p className="text-xs font-semibold uppercase tracking-[0.25em] text-[var(--corbusier-muted)]">
          Task intake
        </p>
        <h2 className="mt-2 text-4xl font-semibold">
          {t('task.create.title')}
        </h2>
        <p className="mt-4 max-w-2xl text-sm leading-7 text-[var(--corbusier-muted)]">
          {t('task.create.description')}
        </p>
      </section>
      <aside className="surface-panel rounded-[var(--corbusier-radius)] p-6">
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
