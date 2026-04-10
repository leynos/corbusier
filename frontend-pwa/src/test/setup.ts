/**
 * Global frontend test setup for Vitest component and unit suites.
 *
 * The module installs jest-dom matchers and provides a no-op `window.scrollTo`
 * implementation for JSDOM-based tests.
 */
import '@testing-library/jest-dom/vitest';

window.scrollTo = () => {};
