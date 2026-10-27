/**
 * Exercise HTTP-backed task flows through the rendered application.
 *
 * Playwright intercepts the API boundary while the application uses its real
 * HTTP gateway and renders the returned task or mapped error state.
 */
import { expect, test } from '@playwright/test';

const taskEnvelope = `{
  "data": {
    "task": {
      "id": "http-task-1",
      "origin": {
        "type": "issue",
        "issue_ref": {
          "provider": "github",
          "repository": "acme/widgets",
          "issue_number": 84
        },
        "metadata": {
          "title": "HTTP-backed task",
          "labels": [],
          "assignees": []
        }
      },
      "state": "draft",
      "created_at": "2026-04-13T00:00:00.000Z",
      "updated_at": "2026-04-13T00:00:00.000Z"
    }
  },
  "error": null,
  "metadata": {
    "request_id": "req-http-1",
    "timestamp": "2026-04-13T00:00:00.000Z",
    "version": "v1"
  },
  "success": true
}`;

test('creates a task over HTTP and loads its detail', async ({ page }) => {
  await page.route('**/api/v1/tasks', async (route) => {
    await route.fulfill({
      body: taskEnvelope,
      contentType: 'application/json',
      status: 201,
    });
  });
  await page.route('**/api/v1/tasks/http-task-1', async (route) => {
    await route.fulfill({
      body: taskEnvelope,
      contentType: 'application/json',
    });
  });
  await page.goto('/tasks/new');
  await page.getByLabel('Repository').fill('acme/widgets');
  await page.getByLabel('Issue number').fill('84');
  await page.getByLabel('Title').fill('HTTP-backed task');

  const createRequest = page.waitForRequest(
    (request) =>
      request.url().endsWith('/api/v1/tasks') && request.method() === 'POST',
  );
  await page.getByRole('button', { name: 'Create task' }).click();
  const request = await createRequest;

  expect(request.postData()).toBe(
    JSON.stringify({
      provider: 'github',
      repository: 'acme/widgets',
      issue_number: 84,
      title: 'HTTP-backed task',
    }),
  );
  expect(request.headers()['idempotency-key']).toMatch(
    /^[0-9a-f]{8}-[0-9a-f]{4}-4[0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}$/i,
  );
  await expect(page.getByText('Task detail')).toBeVisible();
  await expect(
    page.getByRole('heading', { name: 'HTTP-backed task' }),
  ).toBeVisible();

  // Reloading clears the create mutation's query cache and exercises GET.
  const detailRequest = page.waitForRequest(
    (candidate) =>
      candidate.url().endsWith('/api/v1/tasks/http-task-1') &&
      candidate.method() === 'GET',
  );
  await page.goto('/tasks/http-task-1');
  await detailRequest;
  await expect(
    page.getByRole('heading', { name: 'HTTP-backed task' }),
  ).toBeVisible();
});

test('maps an HTTP not-found response to the task not-found view', async ({
  page,
}) => {
  await page.route('**/api/v1/tasks/missing', async (route) => {
    await route.fulfill({
      json: { code: 'not_found', message: 'Task not found' },
      status: 404,
    });
  });

  await page.goto('/tasks/missing');

  await expect(page.getByText('Task not found')).toBeVisible();
  await expect(
    page.getByRole('link', { name: 'Return to task creation' }),
  ).toBeVisible();
});

test('shows the error state for a malformed successful HTTP response', async ({
  page,
}) => {
  await page.route('**/api/v1/tasks/invalid', async (route) => {
    await route.fulfill({
      body: '{',
      contentType: 'application/json',
      status: 200,
    });
  });

  await page.goto('/tasks/invalid');

  await expect(page.getByRole('alert')).toHaveText(
    'Something went wrong loading the task.',
  );
});
