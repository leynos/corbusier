/**
 * Configure Playwright end-to-end runs for the repository-owned frontend PWA.
 *
 * This module exports the default browser test configuration, including device
 * presets and separate fixture and HTTP-mode Vite servers used by test runs.
 */
import { defineConfig, devices } from '@playwright/test';

export default defineConfig({
  testDir: './tests/e2e',
  fullyParallel: true,
  use: {
    baseURL: 'http://127.0.0.1:4173',
    trace: 'on-first-retry',
  },
  webServer: [
    {
      command: 'bun run dev --host 127.0.0.1 --port 4173',
      port: 4173,
      reuseExistingServer: !process.env.CI,
    },
    {
      command: 'bun run dev --host 127.0.0.1 --port 4174',
      env: {
        VITE_CORBUSIER_API_BASE_URL: '/api/v1',
        VITE_TASK_GATEWAY_MODE: 'http',
      },
      port: 4174,
      reuseExistingServer: false,
    },
  ],
  projects: [
    {
      name: 'chromium',
      testIgnore: '**/http-task-flow.spec.ts',
      use: { ...devices['Desktop Chrome'] },
    },
    {
      name: 'http-chromium',
      testMatch: '**/http-task-flow.spec.ts',
      use: { ...devices['Desktop Chrome'], baseURL: 'http://127.0.0.1:4174' },
    },
  ],
});
