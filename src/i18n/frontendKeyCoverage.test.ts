import ts from "typescript";
import { describe, expect, it } from "vitest";

import { hasLocaleKey } from "./i18nTestHelpers";
import en from "./locales/en.json";

const sourceModules = import.meta.glob("../**/*.ts", {
  eager: true,
  import: "default",
  query: "?raw",
}) as Record<string, string>;

const sourceTsxModules = import.meta.glob("../**/*.tsx", {
  eager: true,
  import: "default",
  query: "?raw",
}) as Record<string, string>;

const SOURCE_MODULES = {
  ...sourceModules,
  ...sourceTsxModules,
};

function isIgnoredModule(modulePath: string): boolean {
  return (
    modulePath.includes("/i18n/locales/") ||
    modulePath.endsWith(".test.ts") ||
    modulePath.endsWith(".test.tsx")
  );
}

function collectLiteralTranslationKeys(
  modulePath: string,
  sourceText: string,
): string[] {
  const scriptKind = modulePath.endsWith(".tsx") ? ts.ScriptKind.TSX : ts.ScriptKind.TS;
  const sourceFile = ts.createSourceFile(
    modulePath,
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
    const missingReferences = Object.entries(SOURCE_MODULES)
      .filter(([modulePath]) => !isIgnoredModule(modulePath))
      .flatMap(([modulePath, sourceText]) => {
        const relativePath = modulePath.replace(/^\.\.\//, "");

        return collectLiteralTranslationKeys(modulePath, sourceText)
          .filter((key) => !hasLocaleKey(en, key))
          .map((key) => `${relativePath}: ${key}`);
      })
      .sort();

    expect(missingReferences).toEqual([]);
  });
});
