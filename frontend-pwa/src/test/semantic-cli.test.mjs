/**
 * @file Exercise semantic lint scripts as commands against isolated project
 * fixtures.
 * The fixture copies only the script under test and shares installed packages;
 * it never changes the real frontend configuration or process environment.
 * Run through the frontend Vitest suite to check observable command outcomes.
 */
import { spawnSync } from 'node:child_process';
import {
  copyFileSync,
  mkdirSync,
  mkdtempSync,
  rmSync,
  symlinkSync,
  writeFileSync,
} from 'node:fs';
import { tmpdir } from 'node:os';
import path from 'node:path';
import { afterEach, beforeEach, describe, expect, it } from 'vitest';

const frontendRoot = process.cwd();
let fixtureRoot;

/** Run one copied Bun script with its working directory set to the fixture. */
function runScript(name) {
  const script = path.join(fixtureRoot, 'scripts', name);
  copyFileSync(path.join(frontendRoot, 'scripts', name), script);
  const result = spawnSync('bun', [script], {
    cwd: fixtureRoot,
    encoding: 'utf8',
    timeout: 30_000,
  });
  if (result.error) {
    throw result.error;
  }
  return result;
}

/** Write the semantic-lint configuration consumed by the fixture commands. */
function writeConfig(config) {
  writeFileSync(
    path.join(fixtureRoot, 'tools', 'semantic-lint.config.json'),
    JSON.stringify(config),
  );
}

/** Supply one TSX source file for scripts that inspect component markup. */
function writeComponent(source) {
  writeFileSync(path.join(fixtureRoot, 'src', 'fixture.tsx'), source);
}

describe('semantic lint commands', () => {
  beforeEach(() => {
    fixtureRoot = mkdtempSync(path.join(tmpdir(), 'corbusier-semantic-'));
    for (const directory of ['scripts', 'src', 'tools']) {
      mkdirSync(path.join(fixtureRoot, directory));
    }
    symlinkSync(
      path.join(frontendRoot, 'node_modules'),
      path.join(fixtureRoot, 'node_modules'),
      'junction',
    );
  });

  afterEach(() => {
    rmSync(fixtureRoot, { recursive: true, force: true });
  });

  it('uses the default class limit and accepts an explicit higher limit', () => {
    const classes = Array.from({ length: 17 }, (_, index) => `c${index}`).join(
      ' ',
    );
    writeComponent(`<div className="${classes}" />`);
    writeConfig({});

    const defaultResult = runScript('check-classlist-length.ts');
    expect(defaultResult.status).toBe(1);
    expect(defaultResult.stderr).toContain('17 utilities (max 16)');

    writeConfig({ maxClasslistLength: 17 });
    const configuredResult = runScript('check-classlist-length.ts');
    expect(configuredResult.status).toBe(0);
  });

  it.each([
    0,
    0.5,
    -1,
    '16',
  ])('rejects an invalid class limit %s', (maxClasslistLength) => {
    writeConfig({ maxClasslistLength });
    const result = runScript('check-classlist-length.ts');
    expect(result.status).not.toBe(0);
    expect(result.stderr).toContain('maxClasslistLength');
  });

  it('floors a fractional class limit while preserving a minimum of one', () => {
    writeConfig({ maxClasslistLength: 1.9 });
    writeComponent('<div className="one" />');
    const withinLimit = runScript('check-classlist-length.ts');
    expect(withinLimit.status).toBe(0);

    writeComponent('<div className="one two" />');
    const overLimit = runScript('check-classlist-length.ts');
    expect(overLimit.status).toBe(1);
    expect(overLimit.stderr).toContain('2 utilities (max 1)');
  });

  it('reports repeated stacks without failing by default and can fail explicitly', () => {
    writeComponent(
      '<><div className="flex gap-2 p-2 rounded-md" /><div className="flex gap-2 p-2 rounded-md" /></>',
    );
    writeConfig({});
    const defaultResult = runScript('find-near-duplicate-classes.ts');
    expect(defaultResult.status).toBe(0);
    expect(defaultResult.stderr).toContain('Repeated class stack');

    writeConfig({ nearDuplicateClasses: { failOnViolation: true } });
    const configuredResult = runScript('find-near-duplicate-classes.ts');
    expect(configuredResult.status).toBe(1);
    expect(configuredResult.stderr).toContain('Repeated class stack');
  });

  it.each([
    { minTokenCount: 0 },
    { maxJaccardDistance: 1.1 },
    { minOccurrences: 1.5 },
    { suppressPrefixes: ['safe-', 1] },
    { failOnViolation: 'yes' },
  ])('rejects invalid near-duplicate options %#', (options) => {
    writeConfig({ nearDuplicateClasses: options });
    const result = runScript('find-near-duplicate-classes.ts');
    expect(result.status).not.toBe(0);
    expect(result.stderr).toContain('nearDuplicateClasses');
  });

  it('uses default user-facing attributes when the configured list is empty', () => {
    writeComponent('<button title="Save" />');
    writeConfig({ hardcodedStrings: { userFacingAttributes: [] } });
    const result = runScript('check-hardcoded-strings.ts');
    expect(result.status).toBe(1);
    expect(result.stderr).toContain('user-facing attribute');
  });

  it('honours configured attributes and rejects non-string entries', () => {
    writeComponent('<button title="Save" />');
    writeConfig({ hardcodedStrings: { userFacingAttributes: ['data-label'] } });
    const configuredResult = runScript('check-hardcoded-strings.ts');
    expect(configuredResult.status).toBe(0);

    writeConfig({ hardcodedStrings: { userFacingAttributes: ['title', 7] } });
    const invalidResult = runScript('check-hardcoded-strings.ts');
    expect(invalidResult.status).not.toBe(0);
    expect(invalidResult.stderr).toContain('userFacingAttributes');
  });

  it('uses the default word length and accepts a smaller configured limit', () => {
    writeComponent('<button>a</button>');
    writeConfig({});
    const defaultResult = runScript('check-hardcoded-strings.ts');
    expect(defaultResult.status).toBe(0);

    writeConfig({ hardcodedStrings: { minWordLength: 1 } });
    const configuredResult = runScript('check-hardcoded-strings.ts');
    expect(configuredResult.status).toBe(1);
    expect(configuredResult.stderr).toContain('hard-coded JSX text');
  });

  it('accepts matching locale maps and rejects a non-string catalogue', () => {
    const localeDir = path.join(fixtureRoot, 'src', 'i18n');
    mkdirSync(localeDir);
    writeFileSync(
      path.join(localeDir, 'en-gb.ts'),
      'export const messages = { greeting: "Hello {name}" };\n',
    );
    writeFileSync(
      path.join(localeDir, 'fr.ts'),
      'export const messages = { greeting: "Bonjour {name}" };\n',
    );

    const matchingResult = runScript('check-locale-vars.ts');
    expect(matchingResult.status).toBe(0);
    expect(matchingResult.stdout).toContain('Locale catalogues match');

    writeFileSync(
      path.join(localeDir, 'fr.ts'),
      'export const messages = { greeting: 12 };\n',
    );
    const invalidResult = runScript('check-locale-vars.ts');
    expect(invalidResult.status).not.toBe(0);
    expect(invalidResult.stderr).toContain(
      'Expected exactly one message catalogue export',
    );
  });
});
