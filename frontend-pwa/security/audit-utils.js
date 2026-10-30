/**
 * @file Shared helpers for running `bun audit` and reasoning about advisories.
 *
 * These helpers centralize the JSON parsing and filtering logic used by the
 * security validation scripts. They ensure both the security gate and
 * workspace-specific audit wrappers interpret the CLI output consistently.
 *
 * Cross-link: `scripts/run-audit.mjs` consumes these helpers to enforce the
 * audit exception ledger during dependency audits.
 */

import { spawnSync } from 'node:child_process';
import { z } from 'zod';

const GHSA_PATTERN = /GHSA-[0-9a-z]{4}-[0-9a-z]{4}-[0-9a-z]{4}/i;
const AdvisorySchema = z.looseObject({
  github_advisory_id: z.string().optional(),
  title: z.string().optional(),
  url: z.string().optional(),
});
const AdvisoryMapSchema = z.record(z.string(), z.array(AdvisorySchema));
const AuditObjectSchema = z.record(z.string(), z.unknown());

/**
 * Extract a GHSA identifier from an advisory URL.
 *
 * @param {string} url
 * @returns {string | undefined}
 */
function extractGhsaFromUrl(url) {
  const match = url.match(GHSA_PATTERN);
  return match?.[0];
}

/**
 * Read an advisory's GitHub identifier, falling back to its advisory URL.
 * Invalid records and records without a GHSA identifier produce `undefined`.
 *
 * @param {unknown} advisory
 * @returns {string | undefined}
 *
 * @example
 * extractGithubAdvisoryId({ url: 'https://github.com/advisories/GHSA-abcd-1234-efgh' });
 * // Returns 'GHSA-abcd-1234-efgh'.
 */
function extractGithubAdvisoryId(advisory) {
  const result = AdvisorySchema.safeParse(advisory);
  if (!result.success) {
    return;
  }

  if (result.data.github_advisory_id !== undefined) {
    return result.data.github_advisory_id;
  }

  if (result.data.url !== undefined) {
    return extractGhsaFromUrl(result.data.url);
  }

  return;
}

/**
 * Validate that `spawnSync` exited normally.
 *
 * @param {import('node:child_process').SpawnSyncReturns<string>} result
 * @returns {void}
 */
function assertSpawnSuccess(result) {
  if (result.error) {
    throw result.error;
  }
  if (result.signal) {
    throw new Error(
      `bun audit did not exit normally (signal: ${result.signal})`,
    );
  }
  if (!Number.isInteger(result.status)) {
    throw new TypeError('bun audit did not exit normally');
  }
}

/**
 * Parse the JSON payload emitted by `bun audit --json`.
 *
 * @param {string | null | undefined} raw
 * @returns {Record<string, unknown>}
 */
function parseAuditOutput(raw) {
  const stdout = (raw ?? '').trim();
  if (!stdout) {
    return {};
  }

  try {
    return AuditObjectSchema.parse(JSON.parse(stdout));
  } catch (error) {
    const wrapped = new Error(
      `Failed to parse bun audit JSON: ${error.message}`,
    );
    wrapped.cause = error;
    throw wrapped;
  }
}

/**
 * Run `bun audit --json` and return the parsed payload alongside the exit
 * status. Whitespace-only output is treated as an empty advisory list so that
 * callers can rely on deterministic results even when Bun prints nothing.
 *
 * @returns {{ json: Record<string, unknown>, status: number }}
 *
 * @example
 * const { json, status } = runAuditJson();
 * if (status !== 0) {
 *   throw new Error('bun audit failed');
 * }
 * console.log(Object.keys(json));
 */
export function runAuditJson() {
  const result = spawnSync('bun', ['audit', '--json'], {
    encoding: 'utf8',
    stdio: ['ignore', 'pipe', 'inherit'],
  });

  assertSpawnSuccess(result);
  const status = result.status ?? 1;
  const json = parseAuditOutput(result.stdout);
  return { json, status };
}

/**
 * Convert Bun's audit JSON into a flat array that is easier to filter.
 *
 * @param {Record<string, any>} auditJson Raw JSON payload from `bun audit`.
 * @returns {Array<{
 *   package?: string,
 *   github_advisory_id?: string,
 *   title?: string,
 *   url?: string,
 *   [key: string]: unknown,
 * }>}
 *
 * @example
 * const advisories = collectAdvisories({
 *   lodash: [{ url: 'https://github.com/advisories/GHSA-1' }],
 * });
 * console.log(advisories[0].github_advisory_id);
 */
export function collectAdvisories(auditJson) {
  const root = AuditObjectSchema.safeParse(auditJson);
  if (!root.success) {
    throw new Error('Invalid bun audit advisory payload');
  }

  const entries = Object.hasOwn(root.data, 'advisories')
    ? root.data.advisories
    : root.data;
  const advisories = AdvisoryMapSchema.safeParse(entries);
  if (!advisories.success) {
    throw new Error('Invalid bun audit advisory payload');
  }
  return flattenAdvisories(advisories.data);
}

/**
 * Add each package name and normalized GitHub advisory ID to its records.
 * Records remain in package order and preserve their original fields.
 *
 * @param {Record<string, Array<{ github_advisory_id?: string, url?: string }>>} advisories
 * @returns {Array<Record<string, unknown> & {
 *   package: string,
 *   github_advisory_id?: string
 * }>} Normalized advisory records.
 *
 * @example
 * flattenAdvisories({ pkg: [{ url: 'https://github.com/advisories/GHSA-abcd-1234-efgh' }] });
 * // Returns [{ package: 'pkg', url: 'https://github.com/advisories/GHSA-abcd-1234-efgh',
 * //   github_advisory_id: 'GHSA-abcd-1234-efgh' }].
 */
function flattenAdvisories(advisories) {
  return Object.entries(advisories).flatMap(([packageName, records]) =>
    records.map((advisory) => ({
      ...advisory,
      package: packageName,
      github_advisory_id: extractGithubAdvisoryId(advisory),
    })),
  );
}

/**
 * Split advisories into those whose GitHub advisory IDs are present in the
 * allowed list and those that are unexpected.
 *
 * @param {Array<{ github_advisory_id?: string }>} advisories
 * @param {Iterable<string>} allowedIds
 * @returns {{ expected: typeof advisories, unexpected: typeof advisories }}
 *
 * @example
 * const { expected, unexpected } = partitionAdvisoriesById(
 *   [
 *     { github_advisory_id: 'GHSA-1' },
 *     { github_advisory_id: 'GHSA-2' },
 *   ],
 *   ['GHSA-2'],
 * );
 * console.log(expected.length);
 * console.log(unexpected.length);
 */
export function partitionAdvisoriesById(advisories, allowedIds) {
  const allowed = new Set(allowedIds);
  const expected = [];
  const unexpected = [];

  for (const advisory of advisories) {
    const id = advisory.github_advisory_id;
    if (id && allowed.has(id)) {
      expected.push(advisory);
    } else {
      unexpected.push(advisory);
    }
  }

  return { expected, unexpected };
}

/**
 * Format a single unexpected advisory for stderr output.
 *
 * @param {{ github_advisory_id?: string, title?: string, package?: string }} advisory
 * @returns {string}
 */
function formatAdvisoryLine(advisory) {
  const id = advisory.github_advisory_id ?? 'UNKNOWN';
  const packageName = advisory.package ? ` (${advisory.package})` : '';
  const suffix = advisory.title ? `: ${advisory.title}` : '';
  return `- ${id}${packageName}${suffix}`;
}

/**
 * Report unexpected advisories to stderr.
 *
 * @param {Array<{ github_advisory_id?: string, title?: string, package?: string }>} unexpected
 * @param {string} heading
 * @returns {boolean}
 *
 * @example
 * const hadUnexpected = reportUnexpectedAdvisories(
 *   [{ github_advisory_id: 'GHSA-1', title: 'Example', package: 'lodash' }],
 *   'Unexpected advisories:',
 * );
 * console.log(hadUnexpected);
 */
export function reportUnexpectedAdvisories(unexpected, heading) {
  if (unexpected.length === 0) {
    return false;
  }

  console.error(heading);
  for (const advisory of unexpected) {
    console.error(formatAdvisoryLine(advisory));
  }

  return true;
}
