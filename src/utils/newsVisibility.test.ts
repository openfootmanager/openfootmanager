import { describe, expect, it } from "vitest";

import { isNewsArticleVisible } from "./newsVisibility";

describe("isNewsArticleVisible", () => {
  const today = "2026-02-15T00:00:00+00:00";

  it("shows past and same-day articles", () => {
    expect(isNewsArticleVisible("2026-02-01", today)).toBe(true);
    expect(isNewsArticleVisible("2026-02-15", today)).toBe(true);
  });

  it("shows a same-day RFC3339 article", () => {
    expect(isNewsArticleVisible("2026-02-15T09:30:00+00:00", today)).toBe(true);
  });

  it("hides a future-dated article until its day", () => {
    expect(isNewsArticleVisible("2026-06-03", today)).toBe(false);
  });

  it("treats a missing clock date as no filter", () => {
    expect(isNewsArticleVisible("2026-06-03", undefined)).toBe(true);
  });
});
