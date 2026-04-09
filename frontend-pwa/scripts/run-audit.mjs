/**
 * @file Ensures `bun audit` only fails for advisories covered by the audit
 * exception ledger.
 *
 * Any vulnerabilities without explicit ledger entries are considered fatal.
 */

import { realpathSync } from 'node:fs';
import { resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

import packageJson from '../package.json' with { type: 'json' };
import auditExceptions from '../security/audit-exceptions.json' with {
  type: 'json',
};
import {
  collectAdvisories,
  partitionAdvisoriesById,
  reportUnexpectedAdvisories,
  runAuditJson,
} from '../security/audit-utils.js';

const normalise = (path) =>
  typeof realpathSync.native === 'function'
    ? realpathSync.native(path)
    : realpathSync(path);

const workspacePackageName = packageJson.name;
const workspaceKeys = new Set([
  workspacePackageName,
  workspacePackageName.includes('/')
    ? workspacePackageName.slice(workspacePackageName.lastIndexOf('/') + 1)
    : workspacePackageName,
]);

const unexpectedHeading = 'Unexpected vulnerabilities detected by bun audit:';
const DAY_MS = 24 * 60 * 60 * 1000;

function buildLedgerMaps(currentWorkspaceKeys, auditEntries, referenceDate) {
  if (
    !(referenceDate instanceof Date) ||
    Number.isNaN(referenceDate.getTime())
  ) {
    throw new TypeError('Invalid reference date');
  }

  const ledgerByAdvisory = new Map();
  const allowedIds = [];
  const seenIds = new Set();

  for (const entry of auditEntries) {
    if (entry.id) {
      if (seenIds.has(entry.id)) {
        throw new Error(`Duplicate audit exception id detected: ${entry.id}`);
      }

      seenIds.add(entry.id);
    }

    if (!currentWorkspaceKeys.has(entry.package)) {
      continue;
    }

    ledgerByAdvisory.set(entry.advisory, entry);
    allowedIds.push(entry.advisory);
  }

  return { ledgerByAdvisory, allowedIds };
}

function getLedgerExpiryError(entry, advisoryId, referenceDateValue) {
  if (!entry) {
    return `Audit ledger entry missing for advisory ${advisoryId ?? 'unknown'}.`;
  }

  const entryLabel = entry.id ?? entry.advisory;
  if (!entry.expiresAt) {
    return `Audit exception ${entryLabel} for advisory ${entry.advisory} is missing an expiry date.`;
  }

  const rawExpiry = String(entry.expiresAt).trim();
  const expiryDate = new Date(rawExpiry);
  if (Number.isNaN(expiryDate.valueOf())) {
    return `Audit exception ${entryLabel} for advisory ${entry.advisory} has an invalid expiry date (raw: ${rawExpiry || ''}, expected ISO 8601).`;
  }

  const dateOnlyPattern = /^\d{4}-\d{2}-\d{2}$/;
  const expiryBoundary = dateOnlyPattern.test(rawExpiry)
    ? expiryDate.getTime() + DAY_MS
    : expiryDate.getTime();
  if (expiryBoundary <= referenceDateValue) {
    return `Audit exception ${entryLabel} for advisory ${entry.advisory} expired on ${rawExpiry}.`;
  }

  return null;
}

function collectAdvisoryExpiryErrors(
  advisories,
  ledgerByAdvisory,
  referenceDateValue,
) {
  const errors = [];
  for (const advisory of advisories) {
    const entry = ledgerByAdvisory.get(advisory.github_advisory_id ?? '');
    const error = getLedgerExpiryError(
      entry,
      advisory.github_advisory_id,
      referenceDateValue,
    );
    if (error) {
      errors.push(error);
    }
  }

  return errors;
}

function reportExpiryFailures(expected, ledgerByAdvisory, referenceDateValue) {
  const expiryErrors = collectAdvisoryExpiryErrors(
    expected,
    ledgerByAdvisory,
    referenceDateValue,
  );
  if (expiryErrors.length === 0) {
    return false;
  }

  for (const error of expiryErrors) {
    console.error(error);
  }

  return true;
}

function reportAllowedAdvisories(expected) {
  const advisoryCount = expected.length;
  const suffix = advisoryCount === 1 ? 'advisory' : 'advisories';
  console.info(
    `All reported advisories are covered by the audit exception ledger (${advisoryCount} ${suffix}).`,
  );
  return 0;
}

/**
 * Determine whether the current module is executed as the entry script.
 *
 * @param {ImportMeta} meta
 * @returns {boolean}
 *
 * @example
 * if (isExecutedDirectly(import.meta)) {
 *   console.log('Run from CLI');
 * }
 */
function isExecutedDirectly(meta) {
  const invokedPath = process.argv?.[1];
  if (!invokedPath) {
    return false;
  }

  try {
    const scriptPath = fileURLToPath(meta.url);
    const absoluteInvokedPath = resolve(invokedPath);
    return normalise(scriptPath) === normalise(absoluteInvokedPath);
  } catch {
    return false;
  }
}

/**
 * Evaluate bun audit output and determine the appropriate exit code.
 *
 * @param {{ advisories?: Array<Record<string, unknown>>, status?: number }} payload
 * @param {{ now?: Date }} [options]
 * @returns {number}
 *
 * @example
 * const exitCode = evaluateAudit({ advisories: [], status: 0 });
 * console.log(exitCode);
 */
export function evaluateAudit(payload, options = {}) {
  const referenceDate = options.now ?? new Date();
  const referenceDateValue = referenceDate.getTime();
  const rawAdvisories = payload.advisories ?? [];
  const statusCode = payload.status ?? 0;
  const { ledgerByAdvisory, allowedIds } = buildLedgerMaps(
    workspaceKeys,
    auditExceptions,
    referenceDate,
  );
  const { expected, unexpected } = partitionAdvisoriesById(
    rawAdvisories,
    allowedIds,
  );
  const hasExpiredEntries = reportExpiryFailures(
    expected,
    ledgerByAdvisory,
    referenceDateValue,
  );
  const hasUnexpectedAdvisories = reportUnexpectedAdvisories(
    unexpected,
    unexpectedHeading,
  );

  if (hasExpiredEntries || hasUnexpectedAdvisories) {
    return 1;
  }

  if (expected.length === 0) {
    return statusCode;
  }

  return reportAllowedAdvisories(expected);
}

/**
 * Execute `bun audit` and exit according to {@link evaluateAudit}.
 *
 * @returns {number}
 *
 * @example
 * const exitCode = main();
 * console.log(exitCode);
 */
export function main() {
  const { json, status } = runAuditJson();
  const advisories = collectAdvisories(json);
  return evaluateAudit({ advisories, status });
}

if (isExecutedDirectly(import.meta)) {
  try {
    const exitCode = main();
    process.exit(exitCode);
  } catch (error) {
    console.error(error);
    process.exit(1);
  }
}
