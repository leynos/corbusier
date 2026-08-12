/**
 * Integration tests for the zero-tolerance TypeDoc documentation gate.
 *
 * The suite invokes the production `docs:check` script so its success and
 * diagnostic contracts cannot drift away from the command used in CI.
 */
import { spawnSync } from 'node:child_process';
import { mkdtempSync, rmSync, writeFileSync } from 'node:fs';
import { join } from 'node:path';

const undocumentedSymbol = 'UndocumentedTypeDocFixture';

function runDocsCheck(...args: string[]) {
  return spawnSync('bun', ['run', 'docs:check', ...args], {
    cwd: process.cwd(),
    encoding: 'utf8',
  });
}

describe('docs:check', () => {
  it('accepts the documented production surface', () => {
    const result = runDocsCheck();

    expect(result.error).toBeUndefined();
    expect(result.status).toBe(0);
  }, 30_000);

  it('rejects an undocumented export and names the symbol', () => {
    const fixtureDirectory = mkdtempSync(
      join(process.cwd(), 'src/typedoc-fixture-'),
    );
    const fixturePath = join(fixtureDirectory, 'undocumented-export.ts');
    writeFileSync(fixturePath, `export function ${undocumentedSymbol}() {}`);

    try {
      const result = runDocsCheck(
        '--entryPoints',
        fixturePath,
        '--entryPointStrategy',
        'resolve',
      );
      const diagnostics = `${result.stdout}${result.stderr}`;

      expect(result.error).toBeUndefined();
      expect(result.status).not.toBe(0);
      expect(diagnostics).toContain(undocumentedSymbol);
    } finally {
      rmSync(fixtureDirectory, { recursive: true, force: true });
    }
  }, 30_000);
});
