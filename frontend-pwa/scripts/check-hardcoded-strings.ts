/** @file Lint script that detects hard-coded alphabetic strings in JSX.
 *
 * The linter flags user-facing JSX text and selected JSX attributes
 * unless they are wrapped in `t(...)`, suppressed by semantic-lint
 * directives, or placed inside explicitly ignored regions such as
 * `<code>`, `<pre>`, or `data-i18n-ignore`.
 *
 * This mirrors the general semantic-lint pattern used elsewhere in the
 * repo: TypeScript AST parsing, `Bun.Glob` file discovery, config from
 * `tools/semantic-lint.config.json`, and `file:line:col` output.
 */

import { readFileSync } from 'node:fs';
import { dirname, relative, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

import ts from 'typescript';

interface HardcodedStringsConfig {
  minWordLength: number;
  userFacingAttributes: string[];
}

interface SemanticConfig {
  hardcodedStrings?: Partial<HardcodedStringsConfig>;
}

interface Violation {
  file: string;
  line: number;
  column: number;
  text: string;
  kind: 'jsx-text' | 'attribute';
}

interface LintDirectives {
  readonly disabledLines: Set<number>;
  readonly fileLevelDisabled: boolean;
}

/** Shared state for a single-file analysis pass. */
interface VisitContext {
  filePath: string;
  wordRegex: RegExp;
  attrSet: Set<string>;
  source: ts.SourceFile;
  directives: LintDirectives;
  results: Violation[];
}

const SCRIPT_DIR = dirname(fileURLToPath(import.meta.url));
const PROJECT_ROOT = resolve(SCRIPT_DIR, '..');
const CONFIG_PATH = resolve(PROJECT_ROOT, 'tools/semantic-lint.config.json');

const DEFAULTS: HardcodedStringsConfig = {
  minWordLength: 2,
  userFacingAttributes: [
    'aria-label',
    'aria-description',
    'aria-placeholder',
    'placeholder',
    'title',
    'alt',
  ],
};

/** Load semantic-lint config with repo defaults for this rule. */
function loadConfig(): HardcodedStringsConfig {
  const raw = readFileSync(CONFIG_PATH, 'utf8');
  const config = JSON.parse(raw) as SemanticConfig;
  const section = config.hardcodedStrings;
  return {
    minWordLength:
      Number.isFinite(section?.minWordLength) && section.minWordLength > 0
        ? Math.floor(section.minWordLength)
        : DEFAULTS.minWordLength,
    userFacingAttributes:
      Array.isArray(section?.userFacingAttributes) &&
      section.userFacingAttributes.length > 0
        ? section.userFacingAttributes
        : DEFAULTS.userFacingAttributes,
  };
}

/** Return the TSX files scanned by this rule. */
function getTsxFiles(): string[] {
  const glob = new Bun.Glob('src/**/*.tsx');
  return Array.from(glob.scanSync(PROJECT_ROOT), (filePath) =>
    resolve(PROJECT_ROOT, filePath),
  );
}

/**
 * Return true when the configured minimum word length is valid.
 *
 * @param value Candidate minimum word length.
 * @returns {boolean} Whether the value is a finite positive integer.
 */
function isValidMinLength(value: number): boolean {
  return Number.isFinite(value) && Number.isInteger(value) && value >= 1;
}

/** Build the Unicode-aware word matcher used to detect literals. */
export function buildWordRegex(minLength: number): RegExp {
  if (!isValidMinLength(minLength)) {
    throw new TypeError('minLength must be a finite positive integer');
  }

  return new RegExp(`\\p{L}{${minLength},}`, 'u');
}

/**
 * Return true when the node is a `t(...)` call expression.
 *
 * @param node Candidate AST node.
 * @returns {boolean} Whether the node is a translation call expression.
 */
function isTCallExpression(node: ts.Node): boolean {
  return (
    ts.isCallExpression(node) &&
    ts.isIdentifier(node.expression) &&
    node.expression.text === 't'
  );
}

/** Return true when the node sits inside a `t(…)` call expression. */
function isInsideTCall(node: ts.Node): boolean {
  let cursor = node.parent;
  while (cursor) {
    if (isTCallExpression(cursor)) {
      return true;
    }
    cursor = cursor.parent;
  }
  return false;
}

/** Detect `data-i18n-ignore` on an opening JSX element. */
function hasI18nIgnoreAttribute(element: ts.JsxOpeningLikeElement): boolean {
  return element.attributes.properties.some((attribute) => {
    if (!ts.isJsxAttribute(attribute)) {
      return false;
    }

    return attribute.name.getText() === 'data-i18n-ignore';
  });
}

/** Return true for JSX regions intentionally excluded from linting. */
function isIgnoredJsxElement(node: ts.Node): boolean {
  if (ts.isJsxElement(node)) {
    const tagName = node.openingElement.tagName.getText();
    return (
      tagName === 'code' ||
      tagName === 'pre' ||
      hasI18nIgnoreAttribute(node.openingElement)
    );
  }

  if (ts.isJsxSelfClosingElement(node)) {
    const tagName = node.tagName.getText();
    return (
      tagName === 'code' || tagName === 'pre' || hasI18nIgnoreAttribute(node)
    );
  }

  return false;
}

/** Collect semantic-lint disable directives for line- and file-scope skips. */
function collectLintDirectives(
  sourceText: string,
  source: ts.SourceFile,
): LintDirectives {
  const disabledLines = new Set<number>();
  let fileLevelDisabled = false;
  const seenRanges = new Set<string>();
  const disablePattern =
    /semantic-lint:disable\s+hardcoded-strings(?:\s+(\w+))?/u;

  const markRange = (range: ts.CommentRange): void => {
    const key = `${range.pos}:${range.end}`;
    if (seenRanges.has(key)) {
      return;
    }
    seenRanges.add(key);

    const raw = sourceText.slice(range.pos, range.end);
    const match = disablePattern.exec(raw);
    if (!match) {
      return;
    }

    const line = source.getLineAndCharacterOfPosition(range.pos).line + 1;
    const scope = match[1];

    if (scope === 'file') {
      fileLevelDisabled = true;
      return;
    }

    disabledLines.add(line + 1);
  };

  const scanRanges = (ranges: readonly ts.CommentRange[] | undefined): void => {
    if (!ranges) {
      return;
    }

    for (const range of ranges) {
      markRange(range);
    }
  };

  const visit = (node: ts.Node): void => {
    scanRanges(ts.getLeadingCommentRanges(sourceText, node.pos));
    scanRanges(ts.getTrailingCommentRanges(sourceText, node.end));
    ts.forEachChild(node, visit);
  };

  scanRanges(ts.getLeadingCommentRanges(sourceText, 0));
  visit(source);

  return { disabledLines, fileLevelDisabled };
}

/** Check whether the current node falls under an active lint skip. */
function shouldSkipNode(
  node: ts.Node,
  source: ts.SourceFile,
  directives: LintDirectives,
): boolean {
  if (directives.fileLevelDisabled) {
    return true;
  }

  const line =
    source.getLineAndCharacterOfPosition(node.getStart(source)).line + 1;
  return directives.disabledLines.has(line);
}

/**
 * Return true when the expression is an inline string literal form.
 *
 * @param expr Candidate JSX expression payload.
 * @returns {expr is ts.StringLiteral | ts.NoSubstitutionTemplateLiteral}
 */
function isInlineStringExpression(
  expr: ts.Expression | undefined,
): expr is ts.StringLiteral | ts.NoSubstitutionTemplateLiteral {
  return (
    expr !== undefined &&
    (ts.isStringLiteral(expr) || ts.isNoSubstitutionTemplateLiteral(expr))
  );
}

/**
 * Extract text content from a supported JSX attribute initializer.
 *
 * @param initializer Supported JSX attribute initializer node.
 * @returns {string | undefined} Extracted text when the initializer is static.
 */
function extractAttributeText(
  initializer: ts.JsxAttributeValue,
): string | undefined {
  if (ts.isStringLiteral(initializer)) {
    return initializer.text;
  }

  if (
    ts.isJsxExpression(initializer) &&
    isInlineStringExpression(initializer.expression)
  ) {
    return initializer.expression.text;
  }

  return undefined;
}

/**
 * Visit JSX text nodes and record hard-coded user-facing text violations.
 *
 * @param node JSX text node under analysis.
 * @param ctx Shared single-file analysis context.
 * @returns {void}
 */
function visitJsxText(node: ts.JsxText, ctx: VisitContext): void {
  if (!ctx.wordRegex.test(node.text)) {
    return;
  }

  if (isInsideTCall(node) || shouldSkipNode(node, ctx.source, ctx.directives)) {
    return;
  }

  const { line, character } = ctx.source.getLineAndCharacterOfPosition(
    node.getStart(),
  );
  ctx.results.push({
    file: ctx.filePath,
    line: line + 1,
    column: character + 1,
    text: node.text.trim().slice(0, 40),
    kind: 'jsx-text',
  });
}

/**
 * Visit JSX attributes and record hard-coded user-facing attribute violations.
 *
 * @param node JSX attribute node under analysis.
 * @param ctx Shared single-file analysis context.
 * @returns {void}
 */
function visitJsxAttribute(node: ts.JsxAttribute, ctx: VisitContext): void {
  if (!node.initializer) {
    return;
  }

  const attrName = ts.isIdentifier(node.name)
    ? node.name.text
    : node.name.getText();
  if (!ctx.attrSet.has(attrName)) {
    return;
  }

  const text = extractAttributeText(node.initializer);
  if (text === undefined || !ctx.wordRegex.test(text)) {
    return;
  }

  if (isInsideTCall(node) || shouldSkipNode(node, ctx.source, ctx.directives)) {
    return;
  }

  const { line, character } = ctx.source.getLineAndCharacterOfPosition(
    node.getStart(),
  );
  ctx.results.push({
    file: ctx.filePath,
    line: line + 1,
    column: character + 1,
    text: text.slice(0, 40),
    kind: 'attribute',
  });
}

export function analyseFile(
  filePath: string,
  wordRegex: RegExp,
  attrSet: Set<string>,
  results: Violation[],
): void {
  const sourceText = readFileSync(filePath, 'utf8');
  const source = ts.createSourceFile(
    filePath,
    sourceText,
    ts.ScriptTarget.Latest,
    true,
    ts.ScriptKind.TSX,
  );
  const directives = collectLintDirectives(sourceText, source);
  const ctx: VisitContext = {
    filePath,
    wordRegex,
    attrSet,
    source,
    directives,
    results,
  };

  const visit = (node: ts.Node) => {
    if (isIgnoredJsxElement(node)) return;
    if (ts.isJsxText(node)) {
      visitJsxText(node, ctx);
    }
    if (ts.isJsxAttribute(node)) {
      visitJsxAttribute(node, ctx);
    }
    ts.forEachChild(node, visit);
  };

  visit(source);
}

/** Run the rule across the project and print any violations. */
function main(): void {
  const config = loadConfig();
  const wordRegex = buildWordRegex(config.minWordLength);
  const attrSet = new Set(config.userFacingAttributes);
  const violations: Violation[] = [];

  for (const file of getTsxFiles()) {
    analyseFile(file, wordRegex, attrSet, violations);
  }

  for (const v of violations) {
    const displayPath = relative(PROJECT_ROOT, v.file);
    const label =
      v.kind === 'jsx-text'
        ? 'hard-coded JSX text'
        : `hard-coded "${v.text}" in user-facing attribute`;
    console.error(`${displayPath}:${v.line}:${v.column} ${label}`);
  }

  if (violations.length > 0) {
    console.error(
      `\n${violations.length} hard-coded string${violations.length > 1 ? 's' : ''} found. Wrap in t() for localisation.`,
    );
    process.exitCode = 1;
  }
}

if (import.meta.main) {
  main();
}
