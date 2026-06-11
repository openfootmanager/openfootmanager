import fs from "node:fs";
import path from "node:path";

import ts from "typescript";
import { describe, expect, it } from "vitest";

import { hasLocaleKey } from "./i18nTestHelpers";
import en from "./locales/en.json";

const SRC_DIR = path.resolve(process.cwd(), "src");
const SOURCE_EXTENSIONS = new Set([".ts", ".tsx"]);
const IGNORE_RE =
  /(?:\.test\.|[\\/]i18n[\\/]locales[\\/]|node_modules|dist|src-tauri[\\/]target)/;

function walkSourceFiles(dir: string, collected: string[] = []): string[] {
  for (const entry of fs.readdirSync(dir, { withFileTypes: true })) {
    const fullPath = path.join(dir, entry.name);
    if (IGNORE_RE.test(fullPath)) {
      continue;
    }

    if (entry.isDirectory()) {
      walkSourceFiles(fullPath, collected);
      continue;
    }

    if (SOURCE_EXTENSIONS.has(path.extname(entry.name))) {
      collected.push(fullPath);
    }
  }

  return collected;
}

function collectLiteralTranslationKeys(filePath: string): string[] {
  const sourceText = fs.readFileSync(filePath, "utf8");
  const scriptKind = filePath.endsWith(".tsx") ? ts.ScriptKind.TSX : ts.ScriptKind.TS;
  const sourceFile = ts.createSourceFile(
    filePath,
    sourceText,
    ts.ScriptTarget.Latest,
    true,
    scriptKind,
  );
  const keys = new Set<string>();

  function visit(node: ts.Node): void {
    if (ts.isCallExpression(node)) {
      const expression = node.expression;
      const isTranslationCall =
        (ts.isIdentifier(expression) && expression.text === "t") ||
        (ts.isPropertyAccessExpression(expression) && expression.name.text === "t");
      const firstArgument = node.arguments[0];

      if (isTranslationCall && firstArgument && ts.isStringLiteralLike(firstArgument)) {
        keys.add(firstArgument.text);
      }
    }

    ts.forEachChild(node, visit);
  }

  visit(sourceFile);
  return [...keys];
}

describe("frontend translation coverage", () => {
  it("keeps literal translation key references aligned with the English locale", () => {
    const missingReferences = walkSourceFiles(SRC_DIR)
      .flatMap((filePath) => {
        const relativePath = path.relative(SRC_DIR, filePath);

        return collectLiteralTranslationKeys(filePath)
          .filter((key) => !hasLocaleKey(en, key))
          .map((key) => `${relativePath}: ${key}`);
      })
      .sort();

    expect(missingReferences).toEqual([]);
  });
});
