/**
 * Implement the lightweight i18n runtime for the frontend PWA.
 *
 * This module provides the locale context, translation lookup function, and
 * `useI18n` hook consumed by route and component modules.
 */
import { createContext, type PropsWithChildren, useContext } from 'react';

import { enGbMessages, type MessageKey } from './en-gb';

type Messages = typeof enGbMessages;

const I18nContext = createContext({
  /** BCP 47 tag for the active locale; fixed until further locales land. */
  locale: 'en-GB',
  /** Look up a message by key in the active locale's message table. */
  t: (key: MessageKey) => enGbMessages[key],
});

/**
 * Supply the locale context to descendant components.
 */
export function I18nProvider({ children }: PropsWithChildren) {
  const messages: Messages = enGbMessages;
  const value = {
    locale: 'en-GB',
    t: (key: MessageKey) => messages[key],
  };

  return <I18nContext.Provider value={value}>{children}</I18nContext.Provider>;
}

/**
 * Read the active locale and translation function from context.
 */
export function useI18n() {
  return useContext(I18nContext);
}
