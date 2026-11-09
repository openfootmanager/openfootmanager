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
 * Tracks key fields through local identifiers, named helper arguments and returns within each file.
 * This follows symbols within one source file; imports,
 * dynamic property names, object mutation and indirect or external calls are not followed.
 * Direct parameter returns are call-sensitive; other helper returns are context-insensitive,
 * so a safe call may inherit taint from another call.
 * Array destructuring, quoted destructuring properties, JSX props spreads and backend keys in
 * resolver fallbacks are not tracked.
 * Destructured helper parameters and later rest arguments are not tracked; taint is not cleared
 * on reassignment, and defaultValue properties are checked outside translation options too.
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
  const helpers = new Map<
    ts.Symbol,
    ts.FunctionDeclaration | ts.ArrowFunction | ts.FunctionExpression
  >();
  const returnsKey = new Set<ts.Symbol>();
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
  const returnedParameterIndex = (
    helper: ts.FunctionDeclaration | ts.ArrowFunction | ts.FunctionExpression,
  ): number => {
    const body = helper.body;
    if (
      !body ||
      helper.asteriskToken ||
      helper.modifiers?.some((modifier) => modifier.kind === ts.SyntaxKind.AsyncKeyword)
    )
      return -1;
    const value = ts.isBlock(body)
      ? body.statements.length === 1 && ts.isReturnStatement(body.statements[0])
        ? body.statements[0].expression
        : undefined
      : body;
    if (!value || !ts.isIdentifier(value)) return -1;
    const returned = checker.getSymbolAtLocation(value);
    if (!returned) return -1;
    return helper.parameters.findIndex(
      (parameter) =>
        ts.isIdentifier(parameter.name) &&
        !parameter.initializer &&
        !parameter.dotDotDotToken &&
        checker.getSymbolAtLocation(parameter.name) === returned,
    );
  };
  const containsKey = (node: ts.Node): boolean => {
    if (ts.isIdentifier(node)) return isTainted(node);
    if (ts.isPropertyAccessExpression(node)) return keyName(node.name.text);
    if (ts.isElementAccessExpression(node)) {
      if (node.argumentExpression && ts.isStringLiteral(node.argumentExpression))
        return (
          keyName(node.argumentExpression.text) ||
          (/^(0|[1-9]\d*)$/.test(node.argumentExpression.text) && containsKey(node.expression))
        );
      return containsKey(node.expression);
    }
    if (
      ts.isParenthesizedExpression(node) ||
      ts.isAsExpression(node) ||
      ts.isSatisfiesExpression(node) ||
      ts.isTypeAssertionExpression(node) ||
      ts.isNonNullExpression(node) ||
      ts.isJsxExpression(node)
    )
      return !!node.expression && containsKey(node.expression);
    if (ts.isTemplateExpression(node))
      return node.templateSpans.some((span) => containsKey(span.expression));
    if (ts.isArrayLiteralExpression(node)) return node.elements.some(containsKey);
    if (ts.isSpreadElement(node)) return containsKey(node.expression);
    if (ts.isConditionalExpression(node))
      return containsKey(node.whenTrue) || containsKey(node.whenFalse);
    if (
      ts.isBinaryExpression(node) &&
      [
        ts.SyntaxKind.PlusToken,
        ts.SyntaxKind.BarBarToken,
        ts.SyntaxKind.AmpersandAmpersandToken,
        ts.SyntaxKind.QuestionQuestionToken,
      ].includes(node.operatorToken.kind)
    )
      return containsKey(node.left) || containsKey(node.right);
    if (ts.isCallExpression(node) && ts.isIdentifier(node.expression)) {
      const symbol = checker.getSymbolAtLocation(node.expression);
      if (node.expression.text === "resolveBackendText" && (!symbol || !helpers.has(symbol)))
        return false;
      const helper = symbol && helpers.get(symbol);
      if (helper) {
        const index = returnedParameterIndex(helper);
        if (index >= 0 && node.arguments[index]) return containsKey(node.arguments[index]);
      }
      return !!symbol && returnsKey.has(symbol);
    }
    return false;
  };
  walk(file, (node) => {
    if (ts.isFunctionDeclaration(node) && node.name) {
      const symbol = checker.getSymbolAtLocation(node.name);
      if (symbol) helpers.set(symbol, node);
    }
    if (
      ts.isVariableDeclaration(node) &&
      ts.isIdentifier(node.name) &&
      node.initializer &&
      (ts.isArrowFunction(node.initializer) || ts.isFunctionExpression(node.initializer))
    ) {
      const symbol = checker.getSymbolAtLocation(node.name);
      if (symbol) helpers.set(symbol, node.initializer);
    }
  });
  let previous = -1;
  while (previous !== tainted.size + returnsKey.size) {
    previous = tainted.size + returnsKey.size;
    walk(file, (node) => {
      if (
        ts.isParameter(node) &&
        ts.isIdentifier(node.name) &&
        node.initializer &&
        containsKey(node.initializer)
      )
        mark(node.name);
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
        helpers.get(symbol)?.parameters.forEach((param, index) => {
          if (
            node.arguments[index] &&
            containsKey(node.arguments[index]) &&
            ts.isIdentifier(param.name)
          )
            mark(param.name);
        });
      }
    });
    for (const [symbol, helper] of helpers) {
      if (!helper.body) continue;
      const visitReturn = (node: ts.Node): void => {
        if (node !== helper.body && ts.isFunctionLike(node)) return;
        if (ts.isReturnStatement(node) && node.expression && containsKey(node.expression))
          returnsKey.add(symbol);
        ts.forEachChild(node, visitReturn);
      };
      if (ts.isBlock(helper.body)) visitReturn(helper.body);
      else if (containsKey(helper.body)) returnsKey.add(symbol);
    }
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
    if (ts.isShorthandPropertyAssignment(node) && node.name.text === "defaultValue") {
      const symbol = checker.getShorthandAssignmentValueSymbol(node);
      if (symbol && tainted.has(symbol)) results.push(node.getStart(file).toString());
    }
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
    expect(
      offenders(),
      "Backend keys must use resolveBackendText. For a helper-related false positive, pass a literal directly to t; resolve backend keys before rendering.",
    ).toEqual([]);
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
    ["satisfies expression", "t(feedback.headline_key satisfies string)", true],
    [
      "shorthand default",
      "const defaultValue = feedback.headline_key; t('label', { defaultValue })",
      true,
    ],
    [
      "helper parameter initializer",
      "function render(key = feedback.headline_key) { return t(key); } render()",
      true,
    ],
    ["literal satisfies expression", "t('common.cancel' satisfies string)", false],
    [
      "shorthand literal default",
      "const defaultValue = 'Cancel'; t('label', { defaultValue })",
      false,
    ],
    [
      "literal parameter initializer",
      "function render(key = 'common.cancel') { return t(key); } render()",
      false,
    ],
    ["indexed key array", "t([feedback.headline_key][0], feedback.params)", true],
    ["string indexed key array", "t([feedback.headline_key]['0'], feedback.params)", true],
    ["optional call", "t?.(feedback.headline_key)", true],
    ["Trans key", "<Trans i18nKey={feedback.headline_key} />", true],
    ["multiword direct", "t(msg.sender_role_key)", true],
    ["multiword default", "t('x', {defaultValue: msg.sender_role_key})", true],
    ["local alias", "const x = feedback.headline_key; t(x)", true],
    [
      "helper return",
      "function getKey(f) { return f.headline_key; } t(getKey(feedback), feedback.params)",
      true,
    ],
    ["arrow helper return", "const getKey = (f) => f.detail_key; t(getKey(feedback))", true],
    [
      "chained helper return",
      "const first = (f) => f.detail_key; function second(f) { return first(f); } t(second(feedback))",
      true,
    ],
    [
      "identity helper return",
      "function identity(key) { return key; } t(identity(feedback.headline_key))",
      true,
    ],
    [
      "identity literal after backend key",
      "function identity(x: string) { return x; } const detail = resolveBackendText(identity(feedback.detail_key), '', feedback.params); t(identity('common.cancel'));",
      false,
    ],
    [
      "identity key after literal",
      "function identity(x: string) { return x; } identity('common.cancel'); t(identity(feedback.detail_key));",
      true,
    ],
    [
      "resolved text default",
      "const detail = resolveBackendText(feedback.detail_key, '', feedback.params); t('label', { defaultValue: detail })",
      false,
    ],
    [
      "helper returns resolved text",
      "function render(f) { return resolveBackendText(f.detail_key, '', f.params); } t('label', { defaultValue: render(feedback) })",
      false,
    ],
    [
      "predicate selects literal",
      "const hasKey = !!feedback.headline_key; t(hasKey ? 'label.yes' : 'label.no')",
      false,
    ],
    [
      "conditional returns key",
      "const key = hasKey ? feedback.headline_key : 'label.no'; t(key)",
      true,
    ],
    ["renamed destructuring", "const {headline_key: key} = feedback; t(key)", true],
    [
      "helper parameter",
      "function renderKey(key: string) { return t(key); } renderKey(feedback.headline_key)",
      true,
    ],
    ["i18n call", "i18n.t(feedback.headline_key)", true],
    ["key array", "t([feedback.headline_key, 'fallback.label'])", true],
    [
      "local helper shadows resolver",
      "function resolveBackendText(f) { return f.headline_key; } t(resolveBackendText(feedback))",
      true,
    ],
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
