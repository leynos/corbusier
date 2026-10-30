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

expect.extend(toHaveNoViolations);

declare global {
  var IS_REACT_ACT_ENVIRONMENT: boolean;
}

globalThis.IS_REACT_ACT_ENVIRONMENT = true;
globalThis.scrollTo = () => {};

const matchMediaDescriptor = Object.getOwnPropertyDescriptor(
  globalThis,
  'matchMedia',
);
const existingMatchMedia =
  matchMediaDescriptor?.get?.call(globalThis) ?? matchMediaDescriptor?.value;

if (!existingMatchMedia) {
  globalThis.matchMedia = (query: string): MediaQueryList => {
    const members = new Map<PropertyKey, unknown>([
      ['media', query],
      ['matches', false],
      ['onchange', undefined],
      ['addEventListener', () => {}],
      ['removeEventListener', () => {}],
      ['dispatchEvent', () => true],
      ['addListener', () => {}],
      ['removeListener', () => {}],
    ]);

    // SAFETY: The proxy supplies the MediaQueryList members used by tests and
    // no-op legacy listener methods that JSDOM's replacement still needs.
    return new Proxy(
      {},
      { get: (_target, property) => members.get(property) },
    ) as MediaQueryList;
  };
}
