import { describe, expect, it } from "vitest";

/**
 * Every locale file's text, read raw. `JSON.parse` (and so the bundler's JSON import, and so the
 * app) keeps the LAST of two equal keys in an object and says nothing, so the first translation is
 * dropped without a trace and `localeCoverage` never sees it. The parsed tree cannot show this; only
 * the text can.
 */
const localeSources = import.meta.glob<string>("./locales/*.json", {
  query: "?raw",
  import: "default",
  eager: true,
});

interface Duplicate {
  /** The key's path from the root, e.g. `match.playerRoles`. */
  path: string;
  key: string;
}

/**
 * The keys that appear more than once in the same object of a JSON text. It reads the text once,
 * tracking each open object's keys; a string is a key only where an object expects one, so a value
 * that looks like a key, an escaped quote, or the same key in two different objects is not a hit.
 */
export function findDuplicateKeys(text: string): Duplicate[] {
  const duplicates: Duplicate[] = [];
  type Frame =
    | { kind: "object"; keys: Set<string>; expectKey: boolean; name: string | null }
    | { kind: "array" };
  const stack: Frame[] = [];
  // The key the next `{` is the value of, which names that object in a path.
  let pendingName: string | null = null;
  const pathOf = (): string[] =>
    stack.flatMap((frame) => (frame.kind === "object" && frame.name !== null ? [frame.name] : []));

  let index = 0;
  while (index < text.length) {
    const char = text[index];
    if (char === "{") {
      stack.push({ kind: "object", keys: new Set(), expectKey: true, name: pendingName });
      pendingName = null;
    } else if (char === "[") {
      stack.push({ kind: "array" });
      pendingName = null;
    } else if (char === "}" || char === "]") {
      stack.pop();
    } else if (char === ",") {
      const top = stack[stack.length - 1];
      if (top?.kind === "object") top.expectKey = true;
    } else if (char === '"') {
      let end = index + 1;
      while (text[end] !== '"') end += text[end] === "\\" ? 2 : 1;
      const top = stack[stack.length - 1];
      if (top?.kind === "object" && top.expectKey) {
        const key = text.slice(index + 1, end);
        if (top.keys.has(key)) {
          duplicates.push({ path: [...pathOf(), key].join("."), key });
        }
        top.keys.add(key);
        top.expectKey = false;
        // If an object is this key's value, the key names it.
        pendingName = /^\s*:\s*\{/.test(text.slice(end + 1, end + 40)) ? key : null;
      }
      index = end;
    }
    index += 1;
  }
  return duplicates;
}

describe("findDuplicateKeys", () => {
  /**
   * Given an object that names a key twice
   * When its text is scanned
   * Then the key is reported at its path.
   */
  it("reports a key repeated in one object, by its path", () => {
    const text = '{"match": {"playerRoles": "a", "other": "b", "playerRoles": "c"}}';

    expect(findDuplicateKeys(text)).toEqual([{ path: "match.playerRoles", key: "playerRoles" }]);
  });

  /**
   * Given the same key in two different objects, a key's name used as a value, and an escaped quote
   * When their text is scanned
   * Then none of them is a duplicate.
   */
  it("does not report the same key in different objects, values, or escaped quotes", () => {
    const text =
      '{"a": {"title": "x"}, "b": {"title": "y"}, "c": "title", "d": "say \\"title\\"", "title": "z"}';

    expect(findDuplicateKeys(text)).toEqual([]);
  });

  /**
   * Given a key repeated inside an object that sits in an array
   * When its text is scanned
   * Then it is reported.
   */
  it("reports a repeat inside an object in an array", () => {
    expect(findDuplicateKeys('{"list": [{"k": 1, "k": 2}]}')).toEqual([{ path: "k", key: "k" }]);
  });
});

describe("locale files", () => {
  /**
   * Given every locale file on disk
   * When each is scanned for a key named twice in one object
   * Then none is: a duplicate silently drops the first translation.
   */
  it("name no key twice in the same object", () => {
    const found = Object.entries(localeSources).flatMap(([file, text]) =>
      findDuplicateKeys(text).map((duplicate) => `${file}: ${duplicate.path}`),
    );

    expect(found).toEqual([]);
  });

  /**
   * Given the locale files the scan reads
   * When they are counted
   * Then there are several, so an empty glob cannot make the check above pass by reading nothing.
   */
  it("are found by the scan", () => {
    expect(Object.keys(localeSources).length).toBeGreaterThanOrEqual(12);
  });
});
