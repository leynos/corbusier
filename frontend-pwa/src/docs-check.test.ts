/**
 * Behavioural contract test for the frontend TypeDoc Makefile target.
 *
 * The fake Bun executable records the target's invocation without executing
 * TypeDoc, which belongs to the dependency's own validation surface.
 */
import { spawnSync } from 'node:child_process';
import {
  chmodSync,
  mkdtempSync,
  readFileSync,
  rmSync,
  writeFileSync,
} from 'node:fs';
import { tmpdir } from 'node:os';
import { join, resolve } from 'node:path';

const repositoryRoot = resolve(process.cwd(), '..');

describe('frontend-docs-check', () => {
  it('runs the configured Bun command from the frontend workspace', () => {
    const fixtureDirectory = mkdtempSync(join(tmpdir(), 'corbusier-make-'));
    const fakeBun = join(fixtureDirectory, 'bun');
    const argumentsPath = join(fixtureDirectory, 'arguments');
    const workingDirectoryPath = join(fixtureDirectory, 'working-directory');

    writeFileSync(
      fakeBun,
      [
        '#!/usr/bin/env sh',
        'script_dir=$(dirname "$0")',
        'pwd > "$script_dir/working-directory"',
        'printf "%s\\n" "$@" > "$script_dir/arguments"',
      ].join('\n'),
    );
    chmodSync(fakeBun, 0o755);

    try {
      const result = spawnSync(
        'make',
        ['frontend-docs-check', `BUN=${fakeBun}`],
        {
          cwd: repositoryRoot,
          encoding: 'utf8',
        },
      );

      expect(result.error).toBeUndefined();
      expect(result.status).toBe(0);
      expect(readFileSync(workingDirectoryPath, 'utf8').trim()).toBe(
        join(repositoryRoot, 'frontend-pwa'),
      );
      expect(readFileSync(argumentsPath, 'utf8').trim().split('\n')).toEqual([
        'run',
        'docs:check',
      ]);
    } finally {
      rmSync(fixtureDirectory, { recursive: true, force: true });
    }
  });
});
