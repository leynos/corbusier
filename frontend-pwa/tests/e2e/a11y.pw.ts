/**
 * Playwright accessibility checks for the task creation route.
 *
 * The suite uses `AxeBuilder` against the fixture-backed UI and asserts that
 * the task create page exposes no automated accessibility violations.
 */
import AxeBuilder from '@axe-core/playwright';
import { expect, test } from '@playwright/test';

import { sharedAxeRules } from '../utils/shared-axe-rules';

test.describe('Accessibility', () => {
  test('task create route has no accessibility violations', async ({
    page,
  }) => {
    await page.goto('/tasks/new');
    await expect(page).toHaveURL(/\/tasks\/new$/);
    await expect(
      page.getByRole('heading', { name: 'Create task from issue metadata' }),
    ).toBeVisible();

    const accessibilityScanResults = await new AxeBuilder({ page })
      .options({ rules: sharedAxeRules })
      .analyze();

    expect(accessibilityScanResults.violations).toEqual([]);
  });
});
