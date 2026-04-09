#!/usr/bin/env bun
/**
 * @file Ensures locale catalogues keep placeholder parity with the base locale.
 */

import fs from 'node:fs';
import path from 'node:path';
import process from 'node:process';
import { pathToFileURL } from 'node:url';

type MessageMap = Record<string, string>;
type PlaceholderReport = {
  placeholders: Set<string>;
};

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

function isMessageMap(value: unknown): value is MessageMap {
  if (!value || typeof value !== 'object' || Array.isArray(value)) {
    return false;
  }

  return Object.values(value).every((entry) => typeof entry === 'string');
}

function collectPlaceholders(
  messages: MessageMap,
): Map<string, PlaceholderReport> {
  return new Map(
    Object.entries(messages).map(([key, message]) => [
      key,
      {
        placeholders: new Set(
          message.matchAll(PLACEHOLDER_PATTERN).map((match) => match[1]),
        ),
      },
    ]),
  );
}

function compareMessageKeys(
  baseLocale: string,
  locale: string,
  baseMessages: MessageMap,
  localeMessages: MessageMap,
): boolean {
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

function comparePlaceholders(
  baseLocale: string,
  locale: string,
  baseMessages: MessageMap,
  localeMessages: MessageMap,
): boolean {
  const baseReports = collectPlaceholders(baseMessages);
  const localeReports = collectPlaceholders(localeMessages);
  let hasMismatch = false;

  baseReports.forEach((baseReport, key) => {
    const localeReport = localeReports.get(key);
    if (!localeReport) {
      return;
    }

    const missingPlaceholders = [...baseReport.placeholders].filter(
      (placeholder) => !localeReport.placeholders.has(placeholder),
    );
    const extraPlaceholders = [...localeReport.placeholders].filter(
      (placeholder) => !baseReport.placeholders.has(placeholder),
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

async function main(): Promise<void> {
  if (!fs.existsSync(LOCALES_DIR)) {
    console.error(`[locale-vars] No locale directory at ${LOCALES_DIR}`);
    process.exit(1);
  }

  const localeFiles = await listLocaleFiles();
  if (localeFiles.length === 0) {
    console.error(`[locale-vars] No locale files found under ${LOCALES_DIR}`);
    process.exit(1);
  }

  const baseFile = localeFiles.find(
    (filePath) => path.basename(filePath, '.ts') === BASE_LOCALE,
  );
  if (!baseFile) {
    console.error(
      `[locale-vars] Base locale "${BASE_LOCALE}" not found under ${LOCALES_DIR}`,
    );
    process.exit(1);
  }

  const baseMessages = await loadMessages(baseFile);
  const comparisonFiles = localeFiles.filter(
    (filePath) => filePath !== baseFile,
  );

  if (comparisonFiles.length === 0) {
    console.log(
      '[locale-vars] Only the base locale is present; placeholder parity check skipped',
    );
    return;
  }

  let hasMismatch = false;

  for (const filePath of comparisonFiles) {
    const locale = path.basename(filePath, '.ts');
    const localeMessages = await loadMessages(filePath);
    hasMismatch =
      compareMessageKeys(BASE_LOCALE, locale, baseMessages, localeMessages) ||
      hasMismatch;
    hasMismatch =
      comparePlaceholders(BASE_LOCALE, locale, baseMessages, localeMessages) ||
      hasMismatch;
  }

  if (hasMismatch) {
    console.error('[locale-vars] Locale placeholder validation failed');
    process.exit(1);
  }

  console.log('[locale-vars] Locale catalogues match the base locale');
}

await main();
