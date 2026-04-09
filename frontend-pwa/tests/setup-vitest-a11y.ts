import '@testing-library/jest-dom/vitest';
import { configure } from '@testing-library/dom';
import { cleanup } from '@testing-library/react';
import { toHaveNoViolations } from 'jest-axe';
import { afterEach, expect } from 'vitest';

configure({
  defaultHidden: true,
});

afterEach(() => {
  cleanup();
});

declare global {
  namespace Vi {
    interface JestAssertion<T = unknown> {
      toHaveNoViolations(): T;
    }
  }
}

declare global {
  interface CustomMatchers<R = unknown> {
    toHaveNoViolations(): R;
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
