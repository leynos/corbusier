/**
 * @file Detect repeated and near-duplicate class stacks across TSX source.
 *
 * The script scans frontend `src/` TSX files, normalizes `className` literals,
 * and emits
 * diagnostics for exact or Jaccard-similar utility stacks so semantic classes
 * can be extracted before duplication spreads.
 */
import { readFileSync } from 'node:fs';
import { relative } from 'node:path';

import ts from 'typescript';

type Occurrence = {
  file: string;
  line: number;
  column: number;
  raw: string;
  tokens: string[];
  signature: string;
};

type NearDuplicateOptions = {
  minTokenCount: number;
  maxJaccardDistance: number;
  minOccurrences: number;
  suppressPrefixes: string[];
  failOnViolation: boolean;
};

type SemanticConfig = {
  nearDuplicateClasses?: Partial<NearDuplicateOptions>;
};

type ConceptHint = {
  id: string;
  label: string;
  suggestion: string;
  requiredTokens: string[];
};

const PROJECT_ROOT = process.cwd();
const CONFIG_PATH = 'tools/semantic-lint.config.json';
const DEFAULT_OPTIONS: NearDuplicateOptions = {
  minTokenCount: 4,
  maxJaccardDistance: 0.2,
  minOccurrences: 2,
  suppressPrefixes: [],
  failOnViolation: false,
};

const CONCEPT_HINTS: ConceptHint[] = [
  {
    id: 'button-like',
    label: 'button-like utility stack',
    suggestion:
      'Extract a named button class with @apply instead of repeating inline utilities.',
    requiredTokens: [
      'inline-flex',
      'items-center',
      'px-*',
      'py-*',
      'rounded-*',
      'bg-*',
    ],
  },
  {
    id: 'chip',
    label: 'chip or badge utility stack',
    suggestion: 'Extract a named chip or badge class with @apply.',
    requiredTokens: [
      'inline-flex',
      'items-center',
      'rounded-full',
      'px-*',
      'py-*',
    ],
  },
  {
    id: 'card-surface',
    label: 'card surface utility stack',
    suggestion: 'Extract a named card surface class with @apply.',
    requiredTokens: ['rounded-*', 'p-*', 'bg-*', 'border*'],
  },
  {
    id: 'layout-stack',
    label: 'layout wrapper utility stack',
    suggestion:
      'Extract a named layout helper such as `.stack`, `.cluster`, or `.shell`.',
    requiredTokens: ['flex', 'gap-*', 'items-*'],
  },
];

function loadOptions(): NearDuplicateOptions {
  const raw = readFileSync(CONFIG_PATH, 'utf8');
  const config = JSON.parse(raw) as SemanticConfig;
  return { ...DEFAULT_OPTIONS, ...(config.nearDuplicateClasses ?? {}) };
}

function getTsxFiles(): string[] {
  const glob = new Bun.Glob('src/**/*.tsx');
  return Array.from(glob.scanSync(PROJECT_ROOT));
}

function tokenMatches(pattern: string, token: string): boolean {
  if (pattern.endsWith('*')) {
    const base = pattern.slice(0, -1);
    if (base === 'border') {
      return token === 'border' || token.startsWith('border-');
    }
    return token.startsWith(base);
  }
  return token === pattern;
}

function findConceptHint(tokens: string[]): ConceptHint | null {
  for (const hint of CONCEPT_HINTS) {
    const matches = hint.requiredTokens.every((pattern) =>
      tokens.some((token) => tokenMatches(pattern, token)),
    );
    if (matches) {
      return hint;
    }
  }
  return null;
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

function normalizeTokens(raw: string): string[] {
  return Array.from(
    new Set(
      raw
        .trim()
        .split(/\s+/u)
        .map((token) => token.trim().toLowerCase())
        .filter(Boolean),
    ),
  ).sort();
}

function shouldSuppress(
  tokens: string[],
  prefixes: readonly string[],
): boolean {
  if (prefixes.length === 0 || tokens.length === 0) {
    return false;
  }
  return tokens.every((token) =>
    prefixes.some((prefix) => token.startsWith(prefix)),
  );
}

function isCandidate(
  occurrence: Occurrence,
  options: NearDuplicateOptions,
): boolean {
  return (
    occurrence.tokens.length >= options.minTokenCount &&
    !shouldSuppress(occurrence.tokens, options.suppressPrefixes)
  );
}

function getCandidateOccurrences(
  occurrences: readonly Occurrence[],
  options: NearDuplicateOptions,
): Occurrence[] {
  return occurrences.filter((occurrence) => isCandidate(occurrence, options));
}

/** Return true when the node is a JSX `className` attribute with an initializer. */
function isClassNameAttribute(
  node: ts.Node,
): node is ts.JsxAttribute & { initializer: ts.JsxAttributeValue } {
  return (
    ts.isJsxAttribute(node) &&
    node.name.text === 'className' &&
    node.initializer !== undefined
  );
}

function collectOccurrences(filePath: string): Occurrence[] {
  const sourceText = readFileSync(filePath, 'utf8');
  const source = ts.createSourceFile(
    filePath,
    sourceText,
    ts.ScriptTarget.Latest,
    true,
    ts.ScriptKind.TSX,
  );
  const results: Occurrence[] = [];

  const visit = (node: ts.Node) => {
    if (isClassNameAttribute(node)) {
      const literal = extractStringLiteral(node.initializer);
      if (literal) {
        const tokens = normalizeTokens(literal);
        if (tokens.length > 0) {
          const { line, character } = source.getLineAndCharacterOfPosition(
            node.getStart(),
          );
          results.push({
            file: filePath,
            line: line + 1,
            column: character + 1,
            raw: literal,
            tokens,
            signature: tokens.join(' '),
          });
        }
      }
    }
    ts.forEachChild(node, visit);
  };

  visit(source);
  return results;
}

function jaccardDistance(
  left: readonly string[],
  right: readonly string[],
): number {
  const leftSet = new Set(left);
  const rightSet = new Set(right);
  let intersection = 0;
  for (const token of leftSet) {
    if (rightSet.has(token)) {
      intersection += 1;
    }
  }
  const union = new Set([...leftSet, ...rightSet]).size;
  return union === 0 ? 1 : 1 - intersection / union;
}

function reportExactDuplicates(
  occurrences: readonly Occurrence[],
  options: NearDuplicateOptions,
): number {
  const grouped = new Map<string, Occurrence[]>();
  for (const occurrence of getCandidateOccurrences(occurrences, options)) {
    const bucket = grouped.get(occurrence.signature) ?? [];
    bucket.push(occurrence);
    grouped.set(occurrence.signature, bucket);
  }

  let findings = 0;
  for (const group of grouped.values()) {
    if (group.length < options.minOccurrences) {
      continue;
    }
    findings += 1;
    const hint = findConceptHint(group[0]?.tokens ?? []);
    const locations = group
      .map(
        (entry) =>
          `${relative(PROJECT_ROOT, entry.file)}:${entry.line}:${entry.column}`,
      )
      .join(', ');
    console.error(`Repeated class stack found at ${locations}.`);
    if (hint) {
      console.error(`  Hint: ${hint.label}. ${hint.suggestion}`);
    }
  }
  return findings;
}

function reportNearDuplicates(
  occurrences: readonly Occurrence[],
  options: NearDuplicateOptions,
): number {
  const candidates = getCandidateOccurrences(occurrences, options);
  const unique = Array.from(
    new Map(
      candidates.map((occurrence) => [occurrence.signature, occurrence]),
    ).values(),
  );

  let findings = 0;
  const seen = new Set<string>();
  for (let index = 0; index < unique.length; index += 1) {
    const current = unique[index];
    if (!current) {
      continue;
    }
    for (let nextIndex = index + 1; nextIndex < unique.length; nextIndex += 1) {
      const next = unique[nextIndex];
      if (!next) {
        continue;
      }
      const distance = jaccardDistance(current.tokens, next.tokens);
      if (distance > options.maxJaccardDistance) {
        continue;
      }
      const key = [current.signature, next.signature].sort().join('::');
      if (seen.has(key)) {
        continue;
      }
      seen.add(key);
      findings += 1;
      console.error(
        `Near-duplicate class stacks found at ${relative(PROJECT_ROOT, current.file)}:${current.line}:${current.column} and ${relative(PROJECT_ROOT, next.file)}:${next.line}:${next.column}.`,
      );
    }
  }
  return findings;
}

function main(): void {
  const options = loadOptions();
  const occurrences = getTsxFiles().flatMap((filePath) =>
    collectOccurrences(filePath),
  );
  const exactFindings = reportExactDuplicates(occurrences, options);
  const nearFindings = reportNearDuplicates(occurrences, options);

  if (options.failOnViolation && exactFindings + nearFindings > 0) {
    process.exitCode = 1;
  }
}

if (import.meta.main) {
  main();
}
