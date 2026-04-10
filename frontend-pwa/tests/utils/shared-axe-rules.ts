/**
 * Shared axe rule overrides for frontend accessibility test suites.
 *
 * Both jest-axe and Playwright-based axe runs import this module so automated
 * accessibility checks stay aligned across environments.
 */
export const sharedAxeRules = {
  // Radix focus guards intentionally use `aria-hidden` with tabbable nodes.
  'aria-hidden-focus': { enabled: false },
};
