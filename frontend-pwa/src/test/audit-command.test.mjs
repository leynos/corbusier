/** End-to-end checks for the Bun audit wrapper and its exit status. */
import { spawnSync } from 'node:child_process';
import { chmodSync, mkdtempSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import path from 'node:path';
import { afterAll, beforeAll, describe, expect, it } from 'vitest';

const frontendRoot = process.cwd();
const auditScript = path.join(frontendRoot, 'scripts/run-audit.mjs');

describe('audit command', () => {
  let fakeBin;
  let bun;

  beforeAll(() => {
    const lookup = spawnSync('which', ['bun'], { encoding: 'utf8' });
    if (lookup.status !== 0 || !lookup.stdout.trim()) {
      throw new Error('Bun must be available to exercise the audit command');
    }
    bun = lookup.stdout.trim();
    fakeBin = mkdtempSync(path.join(tmpdir(), 'corbusier-audit-'));
    const executable = path.join(fakeBin, 'bun');
    writeFileSync(
      executable,
      `#!/bin/sh
case "$AUDIT_FIXTURE" in
  clean) printf '%s' '{"advisories":{}}'; exit 0 ;;
  invalid_json) printf '%s' '{'; exit 0 ;;
  malformed) printf '%s' '{"advisories":{"lodash":[{"github_advisory_id":5}]}}'; exit 0 ;;
  unexpected) printf '%s' '{"advisories":{"lodash":[{"github_advisory_id":"GHSA-abcd-efgh-ijkl"}]}}'; exit 0 ;;
  nonzero) printf '%s' '{"advisories":{}}'; exit 7 ;;
  signal) kill -TERM $$ ;;
esac
exit 99
`,
    );
    chmodSync(executable, 0o755);
  });

  afterAll(() => {
    rmSync(fakeBin, { recursive: true, force: true });
  });

  it.each([
    ['clean', 0, ''],
    ['invalid_json', 1, 'Failed to parse bun audit JSON'],
    ['malformed', 1, 'Invalid bun audit advisory payload'],
    ['unexpected', 1, 'Unexpected vulnerabilities detected by bun audit'],
    ['nonzero', 7, ''],
    ['signal', 1, 'bun audit did not exit normally'],
  ])('handles %s audit output', (scenario, status, errorText) => {
    const result = spawnSync(bun, ['run', auditScript], {
      cwd: frontendRoot,
      encoding: 'utf8',
      env: {
        ...process.env,
        AUDIT_FIXTURE: scenario,
        PATH: `${fakeBin}${path.delimiter}${process.env.PATH ?? ''}`,
      },
    });

    expect(result.status).toBe(status);
    if (errorText !== '') {
      expect(result.stderr).toContain(errorText);
    }
  });
});
