/**
 * Configure Vitest for the frontend PWA unit and component suites.
 *
 * This module establishes the JSDOM environment, setup hooks, and test file
 * globs used during standard frontend test runs.
 */
import { defineConfig } from 'vitest/config';

export default defineConfig({
  test: {
    environment: 'jsdom',
    globals: true,
    include: ['src/**/*.test.ts', 'src/**/*.test.tsx', 'src/**/*.test.mjs'],
    setupFiles: ['./src/test/setup.ts'],
  },
});
