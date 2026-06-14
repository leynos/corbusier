/**
 * Type declarations for the jest-axe accessibility testing library.
 *
 * Provides TypeScript types for the AxeRunner function type, the configureAxe
 * factory, the axe runner instance, and the toHaveNoViolations matcher used
 * with Vitest's expect assertion surface in accessibility tests.
 */
declare module 'jest-axe' {
  import type { RawMatcherFn } from '@vitest/expect';
  import type { AxeResults, ContextSpec, RunOptions } from 'axe-core';

  export type AxeRunner = (
    html: ContextSpec,
    options?: RunOptions,
  ) => Promise<AxeResults>;

  export function configureAxe(options?: RunOptions): AxeRunner;

  export const axe: AxeRunner;

  export const toHaveNoViolations: {
    toHaveNoViolations: RawMatcherFn;
  };
}
