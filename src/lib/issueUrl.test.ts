import { describe, expect, it } from "vitest";

import { buildBugReportUrl } from "./issueUrl";

const FORM = {
  whatHappened: "The game froze",
  expected: "The match should have started",
  steps: "1. Advance to matchday",
  gameContext: "Matchday 5, Wanderers FC",
  appVersion: "v0.3.0-nightly · f164fcd",
  os: "linux x86_64",
  resolution: "2560x1440",
};

function paramsOf(url: string): URLSearchParams {
  return new URL(url).searchParams;
}

describe("buildBugReportUrl", () => {
  it("points at the repository's new-issue form", () => {
    const url = new URL(buildBugReportUrl("en", FORM));

    expect(url.origin).toBe("https://github.com");
    expect(url.pathname).toBe("/openfootmanager/openfootmanager/issues/new");
  });

  it("chooses the template for the player's language", () => {
    expect(paramsOf(buildBugReportUrl("de", FORM)).get("template")).toBe("bug_report_de.yml");
    expect(paramsOf(buildBugReportUrl("pt-BR", FORM)).get("template")).toBe("bug_report_ptbr.yml");
    expect(paramsOf(buildBugReportUrl("zh-CN", FORM)).get("template")).toBe("bug_report_zh_cn.yml");
  });

  it("falls back to the English form for a language with no template", () => {
    expect(paramsOf(buildBugReportUrl("xx-YY", FORM)).get("template")).toBe("bug_report.yml");
  });

  it("fills every field the form asks for, under the ids it uses", () => {
    const params = paramsOf(buildBugReportUrl("en", FORM));

    expect(params.get("what-happened")).toBe("The game froze");
    expect(params.get("expected-behavior")).toBe("The match should have started");
    expect(params.get("steps-to-reproduce")).toBe("1. Advance to matchday");
    expect(params.get("game-context")).toBe("Matchday 5, Wanderers FC");
    expect(params.get("version")).toBe("v0.3.0-nightly · f164fcd");
    expect(params.get("os")).toBe("linux x86_64");
    expect(params.get("resolution")).toBe("2560x1440");
  });

  it("never prefills the frequency dropdown", () => {
    // Its options are translated per template, so a value that works in English silently fails
    // everywhere else — GitHub drops an unmatched option without saying so.
    expect(paramsOf(buildBugReportUrl("de", FORM)).has("frequency")).toBe(false);
  });

  it("omits a field the player left blank rather than sending an empty one", () => {
    const params = paramsOf(buildBugReportUrl("en", { ...FORM, steps: "", gameContext: "   " }));

    expect(params.has("steps-to-reproduce")).toBe(false);
    expect(params.has("game-context")).toBe(false);
    expect(params.get("what-happened")).toBe("The game froze");
  });

  it("escapes text that would otherwise break the query string", () => {
    const url = buildBugReportUrl("en", {
      ...FORM,
      whatHappened: "crash on 100% & <tag> #5 ?x=1",
    });

    expect(paramsOf(url).get("what-happened")).toBe("crash on 100% & <tag> #5 ?x=1");
  });

  it("keeps the URL inside what a browser will accept", () => {
    // A very long description must not produce a link the browser silently truncates or refuses;
    // the evidence lives in the bundle, so the text is what gets trimmed.
    const url = buildBugReportUrl("en", { ...FORM, whatHappened: "x".repeat(20_000) });

    expect(url.length).toBeLessThanOrEqual(8_000);
    expect(paramsOf(url).get("what-happened")).toContain("x");
  });

  it("marks a trimmed field so nobody thinks that was the whole report", () => {
    const params = paramsOf(buildBugReportUrl("en", { ...FORM, whatHappened: "y".repeat(20_000) }));

    expect(params.get("what-happened")).toMatch(/…$/);
  });
});
