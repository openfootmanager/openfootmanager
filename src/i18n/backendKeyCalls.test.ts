import ts from "typescript";
import { describe, expect, it } from "vitest";

// Every non-test source file, as text.
const SOURCES = import.meta.glob(
  ["../**/*.{ts,tsx}", "!../**/*.test.{ts,tsx}", "!../**/generated/**"],
  {
    query: "?raw",
    import: "default",
    eager: true,
  },
) as Record<string, string>;

/**
 * Tracks key fields through local identifiers and named helper arguments within each file.
 * This follows symbols within one source file; imports,
 * dynamic property names, object mutation and indirect calls are not followed across files.
 * The two named adapters below are trusted to keep their separately tested fallback checks.
 */
function scanSource(path: string, source: string): string[] {
  const file = ts.createSourceFile(
    path,
    source,
    ts.ScriptTarget.Latest,
    true,
    path.endsWith(".tsx") ? ts.ScriptKind.TSX : ts.ScriptKind.TS,
  );
  const host = ts.createCompilerHost({ noResolve: true, noLib: true });
  host.getSourceFile = (name) => (name === path ? file : undefined);
  const checker = ts.createProgram([path], { noResolve: true, noLib: true }, host).getTypeChecker();
  const tainted = new Set<ts.Symbol>();
  const helpers = new Map<ts.Symbol, readonly ts.ParameterDeclaration[]>();
  const mark = (id: ts.Identifier): void => {
    const symbol = checker.getSymbolAtLocation(id);
    if (symbol) tainted.add(symbol);
  };
  const isTainted = (id: ts.Identifier): boolean => {
    const symbol = checker.getSymbolAtLocation(id);
    return !!symbol && tainted.has(symbol);
  };
  const walk = (node: ts.Node, visit: (node: ts.Node) => void): void => {
    visit(node);
    ts.forEachChild(node, (child) => walk(child, visit));
  };
  const keyName = (name: string): boolean => /^[a-z0-9_]+_key$/.test(name);
  const containsKey = (node: ts.Node): boolean => {
    let found = false;
    walk(node, (part) => {
      if (ts.isIdentifier(part) && isTainted(part)) found = true;
      if (ts.isPropertyAccessExpression(part) && keyName(part.name.text)) found = true;
      if (
        ts.isElementAccessExpression(part) &&
        part.argumentExpression &&
        ts.isStringLiteral(part.argumentExpression) &&
        keyName(part.argumentExpression.text)
      )
        found = true;
    });
    return found;
  };
  walk(file, (node) => {
    if (ts.isFunctionDeclaration(node) && node.name) {
      const symbol = checker.getSymbolAtLocation(node.name);
      if (symbol) helpers.set(symbol, node.parameters);
    }
    if (
      ts.isVariableDeclaration(node) &&
      ts.isIdentifier(node.name) &&
      node.initializer &&
      (ts.isArrowFunction(node.initializer) || ts.isFunctionExpression(node.initializer))
    ) {
      const symbol = checker.getSymbolAtLocation(node.name);
      if (symbol) helpers.set(symbol, node.initializer.parameters);
    }
  });
  let previous = -1;
  while (previous !== tainted.size) {
    previous = tainted.size;
    walk(file, (node) => {
      if (ts.isVariableDeclaration(node)) {
        if (ts.isIdentifier(node.name) && node.initializer && containsKey(node.initializer))
          mark(node.name);
        if (ts.isObjectBindingPattern(node.name)) {
          for (const element of node.name.elements) {
            const property = element.propertyName ?? element.name;
            if (
              ts.isIdentifier(property) &&
              keyName(property.text) &&
              ts.isIdentifier(element.name)
            )
              mark(element.name);
          }
        }
      }
      if (
        ts.isBinaryExpression(node) &&
        node.operatorToken.kind === ts.SyntaxKind.EqualsToken &&
        ts.isIdentifier(node.left) &&
        containsKey(node.right)
      )
        mark(node.left);
      if (ts.isCallExpression(node) && ts.isIdentifier(node.expression)) {
        const symbol = checker.getSymbolAtLocation(node.expression);
        if (!symbol) return;
        helpers.get(symbol)?.forEach((param, index) => {
          if (
            node.arguments[index] &&
            containsKey(node.arguments[index]) &&
            ts.isIdentifier(param.name)
          )
            mark(param.name);
        });
      }
    });
  }
  const inFunction = (node: ts.Node, name: string): boolean => {
    let parent: ts.Node | undefined = node.parent;
    while (parent) {
      if (ts.isFunctionDeclaration(parent)) return parent.name?.text === name;
      parent = parent.parent;
    }
    return false;
  };
  const results: string[] = [];
  walk(file, (node) => {
    if (ts.isCallExpression(node)) {
      const callee = node.expression;
      const isTranslation =
        (ts.isIdentifier(callee) && callee.text === "t") ||
        (ts.isPropertyAccessExpression(callee) &&
          ts.isIdentifier(callee.expression) &&
          callee.expression.text === "i18n" &&
          callee.name.text === "t");
      const first = node.arguments[0];
      let safeCompetition = false;
      if (
        path === "../lib/competitionName.ts" &&
        inFunction(node, "competitionDisplayName") &&
        first?.getText(file) === "comp.name_key"
      ) {
        const options = node.arguments[1];
        safeCompetition =
          !!options &&
          ts.isObjectLiteralExpression(options) &&
          options.properties
            .slice(-1)
            .some(
              (property) =>
                ts.isPropertyAssignment(property) &&
                property.name.getText(file) === "defaultValue" &&
                property.initializer.getText(file) === 'comp.name ?? ""',
            );
      }
      // These two adapters check the translated result before returning it to the player.
      const safeAdapter =
        (path === "../utils/backendI18n.ts" &&
          inFunction(node, "resolve") &&
          node.getText(file) === "i18n.t(key, params ?? {})") ||
        (path === "../lib/nationalTeams.ts" &&
          inFunction(node, "nationalTeamDisplayName") &&
          node.getText(file) === 't(nameKey, { defaultValue: "" })');
      if (isTranslation && first && containsKey(first) && !safeCompetition && !safeAdapter)
        results.push(node.getStart(file).toString());
    }
    if (
      ts.isPropertyAssignment(node) &&
      node.name.getText(file).replace(/['"]/g, "") === "defaultValue" &&
      containsKey(node.initializer)
    )
      results.push(node.getStart(file).toString());
    if (
      ts.isJsxAttribute(node) &&
      node.name.getText(file) === "i18nKey" &&
      node.initializer &&
      containsKey(node.initializer)
    )
      results.push(node.getStart(file).toString());
  });
  return results.map(
    (position) => `${path}:${file.getLineAndCharacterOfPosition(Number(position)).line + 1}`,
  );
}

function offenders(): string[] {
  return Object.entries(SOURCES).flatMap(([path, source]) => scanSource(path, source));
}

describe("backend key fields reach the player through the resolver", () => {
  /**
   * Given every source file
   * When it asks `t(...)` for a backend `*_key` field, or gives one as a `defaultValue`
   * Then it is listed: a missing key there would be drawn raw, as the digest once did.
   */
  it("has no t(<backend *_key>) call and no *_key default value", () => {
    expect(offenders()).toEqual([]);
  }, 60000);

  /**
   * Given the shapes the guard exists for
   * When they are matched
   * Then each is caught, and the sanctioned forms are not.
   */
  it.each([
    ["multiline call", "t(\n feedback.headline_key, feedback.params)", true],
    ["multiline default", "t('label', {defaultValue:\n feedback.headline_key})", true],
    ["template literal", "t(`" + "${" + "feedback.headline_key}`)", true],
    ["generic arrow before sink", "const id = <T>(x: T) => x; t(feedback.headline_key)", true],
    ["element access", "t(feedback['headline_key'])", true],
    ["optional call", "t?.(feedback.headline_key)", true],
    ["Trans key", "<Trans i18nKey={feedback.headline_key} />", true],
    ["multiword direct", "t(msg.sender_role_key)", true],
    ["multiword default", "t('x', {defaultValue: msg.sender_role_key})", true],
    ["local alias", "const x = feedback.headline_key; t(x)", true],
    ["renamed destructuring", "const {headline_key: key} = feedback; t(key)", true],
    [
      "helper parameter",
      "function renderKey(key: string) { return t(key); } renderKey(feedback.headline_key)",
      true,
    ],
    ["i18n call", "i18n.t(feedback.headline_key)", true],
    ["nested default", "t('x', {other: {defaultValue: feedback.headline_key}})", true],
    [
      "safe competition fallback",
      'function competitionDisplayName(comp, t) { return t(comp.name_key, {defaultValue: comp.name ?? ""}); }',
      false,
    ],
    [
      "competition fallback overridden by spread",
      'function competitionDisplayName(comp, t) { return t(comp.name_key, {defaultValue: comp.name ?? "", ...unsafe}); }',
      true,
    ],
    [
      "competition without fallback",
      "function competitionDisplayName(comp, t) { return t(comp.name_key); }",
      true,
    ],
    [
      "unrelated competition call",
      "function competitionDisplayName(comp, t) { return t(feedback.detail_key); }",
      true,
    ],
    ["resolver", "resolveBackendText(feedback.headline_key, '', {})", false],
    ["literal", "t('scouting.confidence')", false],
  ])("detects %s", (name, source, matched) => {
    const path = name === "Trans key" ? "../component.tsx" : "../lib/competitionName.ts";
    expect(scanSource(path, source).length > 0).toBe(matched);
  });
});
