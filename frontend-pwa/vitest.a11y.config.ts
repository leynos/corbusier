/**
 * Configure Vitest for accessibility-focused frontend test runs.
 *
 * This module exports the a11y-specific configuration consumed by
 * `vitest --config vitest.a11y.config.ts`.
 */
import { defineConfig } from 'vitest/config';

export default defineConfig({
  test: {
    include: [
      'tests/**/*.a11y.ts',
      'tests/**/*.a11y.tsx',
      'tests/**/*.a11y.test.ts',
      'tests/**/*.a11y.test.tsx',
    ],
    environment: 'jsdom',
    setupFiles: ['tests/setup-vitest-a11y.ts'],
    globals: true,
    reporters: 'default',
    coverage: {
      enabled: false,
    },
    css: false,
  },
});
