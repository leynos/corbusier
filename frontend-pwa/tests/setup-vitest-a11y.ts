/**
 * Configure Vitest accessibility setup for the frontend PWA test suite.
 *
 * This module installs jest-dom and axe matchers plus the DOM shims required
 * by accessibility-focused component tests.
 */
import '@testing-library/jest-dom/vitest';
import { configure } from '@testing-library/dom';
import { cleanup } from '@testing-library/react';
import type { AxeResults } from 'axe-core';
import { toHaveNoViolations } from 'jest-axe';
import { afterEach, expect } from 'vitest';

configure({
  defaultHidden: true,
});

afterEach(() => {
  cleanup();
});

declare module '@vitest/expect' {
  interface Assertion<T> {
    toHaveNoViolations(this: Assertion<AxeResults>): T;
  }
}

declare module 'vitest' {
  interface Assertion<T> {
    toHaveNoViolations(this: Assertion<AxeResults>): T;
  }
}

expect.extend(
  toHaveNoViolations as unknown as Parameters<typeof expect.extend>[0],
);

(
  globalThis as typeof globalThis & {
    IS_REACT_ACT_ENVIRONMENT: boolean;
  }
).IS_REACT_ACT_ENVIRONMENT = true;

window.scrollTo = () => {};

if (!window.matchMedia) {
  window.matchMedia = (query: string): MediaQueryList =>
    ({
      media: query,
      matches: false,
      onchange: null,
      addEventListener: () => undefined,
      removeEventListener: () => undefined,
      addListener: () => undefined,
      removeListener: () => undefined,
      dispatchEvent: () => true,
    }) as MediaQueryList;
}
