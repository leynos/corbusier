/**
 * Ambient declarations for the untyped `jest-axe` package.
 * Covers `AxeRunner`, `configureAxe`, `axe`, and `toHaveNoViolations`.
 * Consumed by `frontend-pwa/tests/setup-vitest-a11y.ts`.
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
