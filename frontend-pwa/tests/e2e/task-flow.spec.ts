/**
 * End-to-end Playwright coverage for the fixture-backed task flow.
 *
 * The suite exercises create, validation, and not-found states, including the
 * seeded missing-task fixture exported by the fixture gateway module.
 */
import { expect, test } from '@playwright/test';

import { fixtureNotFoundTaskId } from '../../src/task_slice/adapters/fixture/fixture-task-gateway';

test('creates a fixture-backed task and lands on detail', async ({ page }) => {
  await page.goto('/tasks/new');

  await page.getByLabel('Repository').fill('acme/widgets');
  await page.getByLabel('Issue number').fill('84');
  await page.getByLabel('Title').fill('Ship the repository-owned task shell');
  await page.getByRole('button', { name: 'Create task' }).click();

  await expect(page.getByText('Task detail')).toBeVisible();
  await expect(
    page.getByText('Ship the repository-owned task shell'),
  ).toBeVisible();
  await expect(
    page.getByRole('definition').filter({ hasText: 'github/acme/widgets/#84' }),
  ).toBeVisible();
});

test('shows invalid create input errors', async ({ page }) => {
  await page.goto('/tasks/new');

  await page.getByRole('button', { name: 'Create task' }).click();

  await expect(
    page.getByText('Use the repository format owner/repository.'),
  ).toBeVisible();
  await expect(
    page.getByText('Issue number must be a positive integer.'),
  ).toBeVisible();
  await expect(page.getByText('Title is required.')).toBeVisible();
});

test('shows a task not found detail state', async ({ page }) => {
  await page.goto(`/tasks/${fixtureNotFoundTaskId}`);

  await expect(page.getByText('Task not found')).toBeVisible();
  await expect(
    page.getByRole('link', { name: 'Return to task creation' }),
  ).toBeVisible();
});
