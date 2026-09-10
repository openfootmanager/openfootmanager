import { describe, expect, it } from "vitest";

import { isMessageVisible, isNewsArticleVisible } from "./newsVisibility";

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

describe("isMessageVisible", () => {
  const today = "2026-02-15T00:00:00+00:00";

  it("shows past and same-day messages, in either date shape", () => {
    expect(isMessageVisible("2026-02-01", today)).toBe(true);
    expect(isMessageVisible("2026-02-15", today)).toBe(true);
    expect(isMessageVisible("2026-02-15T09:30:00+00:00", today)).toBe(true);
  });

  it("hides a message dated ahead of the clock", () => {
    // The inbox had no such rule before #520: a message dated at a fixture or a
    // deadline would have pinned itself to the top and inflated the badge from
    // the day it was written.
    expect(isMessageVisible("2026-06-03", today)).toBe(false);
  });

  it("treats a missing clock date as no filter", () => {
    expect(isMessageVisible("2026-06-03", undefined)).toBe(true);
  });

  it("shows a message with no date rather than hiding it", () => {
    expect(isMessageVisible(undefined, today)).toBe(true);
  });
});
