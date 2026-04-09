import { configureAxe } from 'jest-axe';

export const axe = configureAxe({
  rules: {
    // Radix focus guards intentionally use `aria-hidden` with tabbable nodes.
    'aria-hidden-focus': { enabled: false },
  },
});
