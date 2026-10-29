import { describe, expect, it } from "vitest";

import { findDuplicateKeys } from "./duplicateKeys.helpers";

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
    expect(findDuplicateKeys('{"list": [{"k": 1, "k": 2}]}')).toEqual([
      { path: "list.k", key: "k" },
    ]);
  });

  /**
   * Given a key written once plainly and once with a unicode escape
   * When their text is scanned
   * Then they are the same key, as JSON.parse reads them, and the repeat is reported.
   */
  it("treats an escaped spelling of a key as the same key", () => {
    expect(findDuplicateKeys('{"playerRoles": 1, "player\\u0052oles": 2}')).toEqual([
      { path: "playerRoles", key: "playerRoles" },
    ]);
  });

  /**
   * Given a file cut off in the middle of a string, as a truncated or badly merged one is
   * When its text is scanned
   * Then it says the string never ends, instead of reading past the end of the text.
   */
  it("throws on a string that never closes", () => {
    expect(() => findDuplicateKeys('{"a": "b')).toThrow(/unterminated string/);
    expect(() => findDuplicateKeys('{"a": "b\\')).toThrow(/unterminated string/);
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
