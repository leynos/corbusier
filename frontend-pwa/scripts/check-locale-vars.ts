#!/usr/bin/env bun
/**
 * @file Ensures locale catalogues keep placeholder parity with the base locale.
 */

import fs from 'node:fs';
import path from 'node:path';
import process from 'node:process';
import { pathToFileURL } from 'node:url';

type MessageMap = Record<string, string>;
type MessagePlaceholders = Map<string, Set<string>>;

const BASE_LOCALE = process.env.I18N_BASE_LOCALE ?? 'en-gb';
const LOCALES_DIR = path.resolve(process.cwd(), 'src/i18n');
const PLACEHOLDER_PATTERN = /\{([a-zA-Z0-9_]+)\}/g;

async function listLocaleFiles(): Promise<string[]> {
  return fs
    .readdirSync(LOCALES_DIR, { withFileTypes: true })
    .filter((entry) => entry.isFile() && entry.name.endsWith('.ts'))
    .map((entry) => path.join(LOCALES_DIR, entry.name))
    .sort();
}

async function loadMessages(filePath: string): Promise<MessageMap> {
  const module = await import(pathToFileURL(filePath).href);
  const exports = Object.values(module).filter(isMessageMap);

  if (exports.length !== 1) {
    throw new Error(
      `[locale-vars] Expected exactly one message catalogue export in ${path.basename(filePath)}.`,
    );
  }

  return exports[0];
}

/**
 * Return true when the value is a non-null object and not an array.
 *
 * @param value - Candidate value to inspect.
 * @returns {value is Record<string, unknown>} Whether the value is a plain object candidate.
 */
function isNonArrayObject(value: unknown): value is Record<string, unknown> {
  return value !== null && typeof value === 'object' && !Array.isArray(value);
}

function isMessageMap(value: unknown): value is MessageMap {
  if (!isNonArrayObject(value)) {
    return false;
  }

  return Object.values(value).every((entry) => typeof entry === 'string');
}

function collectPlaceholders(messages: MessageMap): MessagePlaceholders {
  return new Map(
    Object.entries(messages).map(([key, message]) => [
      key,
      new Set(message.matchAll(PLACEHOLDER_PATTERN).map((match) => match[1])),
    ]),
  );
}

export type LocaleComparison = {
  /** Base locale identifier used for comparisons. */
  baseLocale: string;
  /** Locale identifier currently being compared. */
  locale: string;
  /** Message catalogue loaded from the base locale. */
  baseMessages: MessageMap;
  /** Message catalogue loaded from the locale under test. */
  localeMessages: MessageMap;
};

function compareMessageKeys(comparison: LocaleComparison): boolean {
  const { baseLocale, locale, baseMessages, localeMessages } = comparison;
  const baseKeys = new Set(Object.keys(baseMessages));
  const localeKeys = new Set(Object.keys(localeMessages));
  const missingKeys = [...baseKeys].filter((key) => !localeKeys.has(key));
  const extraKeys = [...localeKeys].filter((key) => !baseKeys.has(key));

  if (missingKeys.length === 0 && extraKeys.length === 0) {
    return false;
  }

  console.error(
    `[locale-vars] ${locale} message key mismatch vs ${baseLocale}`,
  );
  if (missingKeys.length > 0) {
    console.error(`  missing keys: ${missingKeys.join(', ')}`);
  }
  if (extraKeys.length > 0) {
    console.error(`  extra keys: ${extraKeys.join(', ')}`);
  }
  return true;
}

function comparePlaceholders(comparison: LocaleComparison): boolean {
  const { baseLocale, locale, baseMessages, localeMessages } = comparison;
  const baseReports = collectPlaceholders(baseMessages);
  const localeReports = collectPlaceholders(localeMessages);
  let hasMismatch = false;

  baseReports.forEach((basePlaceholders, key) => {
    const localePlaceholders = localeReports.get(key);
    if (!localePlaceholders) {
      return;
    }

    const missingPlaceholders = [...basePlaceholders].filter(
      (placeholder) => !localePlaceholders.has(placeholder),
    );
    const extraPlaceholders = [...localePlaceholders].filter(
      (placeholder) => !basePlaceholders.has(placeholder),
    );

    if (missingPlaceholders.length === 0 && extraPlaceholders.length === 0) {
      return;
    }

    hasMismatch = true;
    console.error(
      `[locale-vars] ${locale}:${key} placeholder mismatch vs ${baseLocale}`,
    );
    if (missingPlaceholders.length > 0) {
      console.error(
        `  missing: ${missingPlaceholders.map((name) => `{${name}}`).join(', ')}`,
      );
    }
    if (extraPlaceholders.length > 0) {
      console.error(
        `  extra:   ${extraPlaceholders.map((name) => `{${name}}`).join(', ')}`,
      );
    }
  });

  return hasMismatch;
}

/**
 * Resolve the locale file list after validating the locale directory.
 *
 * @returns {Promise<string[]>} Non-empty locale file list.
 */
async function resolveLocaleFiles(): Promise<string[]> {
  if (!fs.existsSync(LOCALES_DIR)) {
    console.error(`[locale-vars] No locale directory at ${LOCALES_DIR}`);
    process.exit(1);
  }

  const localeFiles = await listLocaleFiles();
  if (localeFiles.length === 0) {
    console.error(`[locale-vars] No locale files found under ${LOCALES_DIR}`);
    process.exit(1);
  }

  return localeFiles;
}

/**
 * Resolve the base locale file from the discovered locale list.
 *
 * @param localeFiles - Locale files discovered under the locale directory.
 * @returns {string} Base locale file path.
 */
function resolveBaseFile(localeFiles: string[]): string {
  const baseFile = localeFiles.find(
    (filePath) => path.basename(filePath, '.ts') === BASE_LOCALE,
  );
  if (!baseFile) {
    console.error(
      `[locale-vars] Base locale "${BASE_LOCALE}" not found under ${LOCALES_DIR}`,
    );
    process.exit(1);
  }

  return baseFile;
}

/**
 * Compare every non-base locale against the base locale catalogue.
 *
 * @param baseMessages - Message catalogue loaded from the base locale file.
 * @param comparisonFiles - Locale files to compare against the base locale.
 * @returns {Promise<boolean>} True when any mismatch is detected.
 */
async function compareLocales(
  baseMessages: MessageMap,
  comparisonFiles: string[],
): Promise<boolean> {
  let hasMismatch = false;

  for (const filePath of comparisonFiles) {
    const locale = path.basename(filePath, '.ts');
    const localeMessages = await loadMessages(filePath);
    const comparison: LocaleComparison = {
      baseLocale: BASE_LOCALE,
      locale,
      baseMessages,
      localeMessages,
    };
    hasMismatch = compareMessageKeys(comparison) || hasMismatch;
    hasMismatch = comparePlaceholders(comparison) || hasMismatch;
  }

  return hasMismatch;
}

/**
 * Check every locale against the base catalogue and exit on mismatch.
 *
 * @param baseMessages - Message catalogue loaded from the base locale file.
 * @param comparisonFiles - Locale files to compare against the base locale.
 * @returns {Promise<void>}
 */
async function checkAllLocales(
  baseMessages: MessageMap,
  comparisonFiles: string[],
): Promise<void> {
  if (comparisonFiles.length === 0) {
    console.log(
      '[locale-vars] Only the base locale is present; placeholder parity check skipped',
    );
    return;
  }

  const hasMismatch = await compareLocales(baseMessages, comparisonFiles);

  if (hasMismatch) {
    console.error('[locale-vars] Locale placeholder validation failed');
    process.exit(1);
  }

  console.log('[locale-vars] Locale catalogues match the base locale');
}

async function main(): Promise<void> {
  const localeFiles = await resolveLocaleFiles();
  const baseFile = resolveBaseFile(localeFiles);
  const baseMessages = await loadMessages(baseFile);
  const comparisonFiles = localeFiles.filter(
    (filePath) => filePath !== baseFile,
  );
  await checkAllLocales(baseMessages, comparisonFiles);
}

await main();
