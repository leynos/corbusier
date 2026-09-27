/**
 * @file Behavioural contract test for the frontend TypeDoc Makefile target.
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

function createFakeBun(fixtureDirectory: string, exitCode: number) {
  const fakeBun = join(fixtureDirectory, 'bun');
  writeFileSync(
    fakeBun,
    [
      '#!/usr/bin/env sh',
      'script_dir=$(dirname "$0")',
      'pwd > "$script_dir/working-directory"',
      'printf "%s\\n" "$@" > "$script_dir/arguments"',
      `exit ${exitCode}`,
    ].join('\n'),
  );
  chmodSync(fakeBun, 0o755);

  return fakeBun;
}

function runDocsCheck(fakeBun: string) {
  return spawnSync('make', ['frontend-docs-check', `BUN=${fakeBun}`], {
    cwd: repositoryRoot,
    encoding: 'utf8',
  });
}

describe('frontend-docs-check', () => {
  it('runs the configured Bun command from the frontend workspace', () => {
    const fixtureDirectory = mkdtempSync(join(tmpdir(), 'corbusier-make-'));

    try {
      const fakeBun = createFakeBun(fixtureDirectory, 0);
      const argumentsPath = join(fixtureDirectory, 'arguments');
      const workingDirectoryPath = join(fixtureDirectory, 'working-directory');

      const result = runDocsCheck(fakeBun);

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

  it('propagates a failing Bun command', () => {
    const fixtureDirectory = mkdtempSync(join(tmpdir(), 'corbusier-make-'));

    try {
      const fakeBun = createFakeBun(fixtureDirectory, 23);
      const result = runDocsCheck(fakeBun);

      expect(result.error).toBeUndefined();
      expect(result.status).not.toBe(0);
    } finally {
      rmSync(fixtureDirectory, { recursive: true, force: true });
    }
  });
});
