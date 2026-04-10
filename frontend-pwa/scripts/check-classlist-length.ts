/**
 * @file Lint script that enforces a configurable maximum `className` length in
 * TSX source.
 *
 * The script scans JSX attributes in frontend `src/` TSX files, reads
 * `SemanticConfig.maxClasslistLength` from `tools/semantic-lint.config.json`,
 * and emits `file:line:column` diagnostics before exiting non-zero when any
 * class list exceeds the configured threshold.
 */
import { readFileSync } from 'node:fs';
import { relative } from 'node:path';

import ts from 'typescript';

interface SemanticConfig {
  maxClasslistLength?: number;
}

interface Violation {
  file: string;
  line: number;
  column: number;
  length: number;
}

const CONFIG_PATH = 'tools/semantic-lint.config.json';
const PROJECT_ROOT = process.cwd();

function loadThreshold(): number {
  const raw = readFileSync(CONFIG_PATH, 'utf8');
  const config = JSON.parse(raw) as SemanticConfig;
  const value = config.maxClasslistLength;
  return typeof value === 'number' && Number.isFinite(value) && value > 0
    ? Math.floor(value)
    : 16;
}

function getTsxFiles(): string[] {
  const glob = new Bun.Glob('src/**/*.tsx');
  return Array.from(glob.scanSync(PROJECT_ROOT));
}

function extractStringLiteral(
  initializer: ts.JsxAttributeValue,
): string | null {
  if (ts.isStringLiteral(initializer)) {
    return initializer.text;
  }
  if (ts.isJsxExpression(initializer) && initializer.expression) {
    const expression = initializer.expression;
    if (
      ts.isStringLiteral(expression) ||
      ts.isNoSubstitutionTemplateLiteral(expression)
    ) {
      return expression.text;
    }
  }
  return null;
}

/** Return true when the node is a JSX `className` attribute with an initializer. */
function isClassNameAttribute(node: ts.Node): node is ts.JsxAttribute {
  return (
    ts.isJsxAttribute(node) &&
    node.name.text === 'className' &&
    node.initializer != null
  );
}

function analyseFile(
  filePath: string,
  maxLength: number,
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

  const visit = (node: ts.Node) => {
    if (isClassNameAttribute(node)) {
      const literal = extractStringLiteral(node.initializer);
      if (literal) {
        const count = literal.trim().split(/\s+/u).filter(Boolean).length;
        if (count > maxLength) {
          const { line, character } = source.getLineAndCharacterOfPosition(
            node.getStart(),
          );
          results.push({
            file: filePath,
            line: line + 1,
            column: character + 1,
            length: count,
          });
        }
      }
    }
    ts.forEachChild(node, visit);
  };

  visit(source);
}

function main(): void {
  const maxLength = loadThreshold();
  const violations: Violation[] = [];

  for (const file of getTsxFiles()) {
    analyseFile(file, maxLength, violations);
  }

  for (const violation of violations) {
    const displayPath = relative(PROJECT_ROOT, violation.file);
    console.error(
      `${displayPath}:${violation.line}:${violation.column} className has ${violation.length} utilities (max ${maxLength}).`,
    );
  }

  if (violations.length > 0) {
    process.exitCode = 1;
  }
}

main();
