/**
 * @file Shared helpers for running `bun audit` and reasoning about advisories.
 *
 * These helpers centralise the JSON parsing and filtering logic used by the
 * security validation scripts. They ensure both the security gate and
 * workspace-specific audit wrappers interpret the CLI output consistently.
 *
 * Cross-link: `scripts/run-audit.mjs` consumes these helpers to enforce the
 * audit exception ledger during dependency audits.
 */

import { spawnSync } from 'node:child_process';

const GHSA_PATTERN = /GHSA-[0-9a-z]{4}-[0-9a-z]{4}-[0-9a-z]{4}/i;

function extractGithubAdvisoryId(advisory) {
  if (!advisory || typeof advisory !== 'object') {
    return null;
  }

  if (typeof advisory.github_advisory_id === 'string') {
    return advisory.github_advisory_id;
  }

  if (typeof advisory.url === 'string') {
    const match = advisory.url.match(GHSA_PATTERN);
    if (match) {
      return match[0];
    }
  }

  return null;
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

  if (result.error) {
    throw result.error;
  }

  const status = result.status ?? 0;
  const stdout = result.stdout ? result.stdout.trim() : '';
  if (!stdout) {
    return { json: {}, status };
  }

  try {
    return { json: JSON.parse(stdout), status };
  } catch (error) {
    error.message = `Failed to parse bun audit JSON: ${error.message}`;
    throw error;
  }
}

/**
 * Resolve Bun's advisory JSON to the list of package-name entry pairs.
 *
 * @param {Record<string, unknown>} auditJson Raw JSON payload from `bun audit`.
 * @returns {Array<[string, unknown]>}
 *
 * @example
 * const entries = resolveAdvisoryEntries({
 *   advisories: { lodash: [{ url: 'https://github.com/advisories/GHSA-1' }] },
 * });
 * console.log(entries[0][0]);
 */
/**
 * Return true when the audit JSON exposes an advisories object map.
 *
 * @param {unknown} auditJson Candidate audit JSON payload.
 * @returns {boolean}
 */
function hasAdvisoryMap(auditJson) {
  return (
    auditJson != null &&
    typeof auditJson === 'object' &&
    auditJson.advisories != null &&
    typeof auditJson.advisories === 'object'
  );
}

function resolveAdvisoryEntries(auditJson) {
  if (hasAdvisoryMap(auditJson)) {
    return Object.entries(auditJson.advisories);
  }

  return Object.entries(auditJson ?? {});
}

/**
 * Expand one package's advisory group into a flat array with derived metadata.
 *
 * @param {string} packageName Package name attached to each advisory.
 * @param {unknown} value Candidate advisory collection for the package.
 * @returns {Array<Record<string, unknown>>}
 *
 * @example
 * const advisories = expandAdvisoryGroup('lodash', [
 *   { url: 'https://github.com/advisories/GHSA-1' },
 * ]);
 * console.log(advisories[0].package);
 */
function expandAdvisoryGroup(packageName, value) {
  if (!Array.isArray(value)) {
    return [];
  }

  return value.map((advisory) => ({
    ...advisory,
    package: packageName,
    github_advisory_id: extractGithubAdvisoryId(advisory),
  }));
}

/**
 * Convert Bun's audit JSON into a flat array that is easier to filter.
 *
 * @param {Record<string, unknown>} auditJson Raw JSON payload from `bun audit`.
 * @returns {Array<Record<string, unknown>>}
 *
 * @example
 * const advisories = collectAdvisories({
 *   lodash: [{ url: 'https://github.com/advisories/GHSA-1' }],
 * });
 * console.log(advisories[0].github_advisory_id);
 */
export function collectAdvisories(auditJson) {
  return resolveAdvisoryEntries(auditJson).flatMap(([packageName, value]) =>
    expandAdvisoryGroup(packageName, value),
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
    const id = advisory.github_advisory_id ?? 'UNKNOWN';
    const packageName = advisory.package ? ` (${advisory.package})` : '';
    const suffix = advisory.title ? `: ${advisory.title}` : '';
    console.error(`- ${id}${packageName}${suffix}`);
  }

  return true;
}
