import { describe, expect, it } from "vitest";

import { SUPPORTED_LANGUAGES } from "../i18n";
import {
  BUG_REPORT_TEMPLATE_BY_LOCALE,
  PREFILLED_FIELD_IDS,
  bugReportTemplateFor,
} from "./issueTemplates";

/**
 * The app builds `issues/new?template=<file>&<field-id>=<value>` URLs. GitHub silently ignores a
 * query parameter that does not name a real field, and silently serves the default template for a
 * file that does not exist — so every way this contract can break is invisible at runtime. These
 * tests read the actual template files instead of trusting the map.
 *
 * Read raw through Vite rather than `node:fs`: the repo has no `@types/node`, so importing it
 * would need a hand-written ambient declaration for one call.
 */
const templateSources = import.meta.glob("../../.github/ISSUE_TEMPLATE/*.yml", {
  query: "?raw",
  import: "default",
  eager: true,
}) as Record<string, string>;

function sourceFor(fileName: string): string | undefined {
  const entry = Object.entries(templateSources).find(([path]) =>
    path.endsWith(`/${fileName}`),
  );
  return entry?.[1];
}

/** Field ids, in order. Matches `    id: foo` — two levels of indent, inside a `body:` item. */
function fieldIdsIn(source: string): string[] {
  return [...source.matchAll(/^\s{4}id:\s*(\S+)\s*$/gm)].map(
    (match) => match[1],
  );
}

describe("bug report issue templates", () => {
  it("reads the real template files", () => {
    // Guards the glob itself: if it ever resolves to nothing, every other test here would pass
    // vacuously while asserting about an empty set.
    expect(Object.keys(templateSources).length).toBeGreaterThan(0);
    expect(sourceFor("bug_report.yml")).toContain("id: what-happened");
  });

  it("ships a template for every locale the game runs in", () => {
    const missing = SUPPORTED_LANGUAGES.map(({ code }) => code).filter(
      (code) => sourceFor(bugReportTemplateFor(code)) === undefined,
    );
    expect(missing).toEqual([]);
  });

  it("exposes every prefilled field id in every language", () => {
    const gaps = Object.entries(BUG_REPORT_TEMPLATE_BY_LOCALE).flatMap(
      ([locale, file]) => {
        const source = sourceFor(file);
        if (source === undefined) return [];
        const ids = fieldIdsIn(source);
        return PREFILLED_FIELD_IDS.filter((id) => !ids.includes(id)).map(
          (id) => `${locale} (${file}) is missing "${id}"`,
        );
      },
    );
    expect(gaps).toEqual([]);
  });

  it("keeps the field ids identical across translations", () => {
    // Prefill is positional only in the sense that it is keyed by id; a translator who renames an
    // id breaks one language and nothing else, which is exactly the kind of bug that survives
    // review. Compare each translation against English.
    const english = fieldIdsIn(sourceFor("bug_report.yml") ?? "");
    const divergent = Object.entries(BUG_REPORT_TEMPLATE_BY_LOCALE)
      .filter(([, file]) => file !== "bug_report.yml")
      .flatMap(([locale, file]) => {
        const source = sourceFor(file);
        if (source === undefined) return [];
        const ids = fieldIdsIn(source);
        return ids.join(",") === english.join(",")
          ? []
          : [`${locale} (${file}): ${ids.join(",")}`];
      });
    expect(divergent).toEqual([]);
  });

  it("falls back to English for a locale with no template of its own", () => {
    expect(bugReportTemplateFor("xx-YY")).toBe("bug_report.yml");
  });
});

describe("issue template locale coverage", () => {
  // `SUPPORTED_LANGUAGES` has no gate of its own — adding a locale to it touches nothing that
  // would go red. A 13th language can therefore ship with translated menus and an English-only
  // set of issue forms, which is how a player ends up reporting a bug in a language they were
  // never asked to write in.
  const FAMILIES = ["bug_report", "feedback", "session_report"] as const;

  it("ships all three forms in every supported language", () => {
    const missing = SUPPORTED_LANGUAGES.flatMap(({ code }) => {
      const suffix = BUG_REPORT_TEMPLATE_BY_LOCALE[code]?.replace(
        /^bug_report/,
        "",
      );
      if (suffix === undefined) return [`${code}: no template mapping`];
      return FAMILIES.filter(
        (family) => sourceFor(`${family}${suffix}`) === undefined,
      ).map((family) => `${code}: missing ${family}${suffix}`);
    });
    expect(missing).toEqual([]);
  });
});
