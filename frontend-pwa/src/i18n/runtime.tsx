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
  locale: 'en-GB',
  t: (key: MessageKey) => enGbMessages[key],
});

export function I18nProvider({ children }: PropsWithChildren) {
  const messages: Messages = enGbMessages;
  const value = {
    locale: 'en-GB',
    t: (key: MessageKey) => messages[key],
  };

  return <I18nContext.Provider value={value}>{children}</I18nContext.Provider>;
}

export function useI18n() {
  return useContext(I18nContext);
}
