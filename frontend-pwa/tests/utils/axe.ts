/**
 * Provide a preconfigured jest-axe instance for accessibility tests.
 *
 * Consumers should import `axe` from this module so `configureAxe` overrides,
 * including the `aria-hidden-focus` exception for Radix focus guards, stay
 * centralized in one place.
 */
import { configureAxe } from 'jest-axe';

export const axe = configureAxe({
  rules: {
    // Radix focus guards intentionally use `aria-hidden` with tabbable nodes.
    'aria-hidden-focus': { enabled: false },
  },
});
