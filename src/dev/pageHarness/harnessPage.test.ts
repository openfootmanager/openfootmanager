import { describe, expect, it } from "vitest";

import { HARNESS_PAGES, readHarnessLanguage, readHarnessPage } from "./harnessPage";

describe("readHarnessLanguage", () => {
  /** Given ?lang=de, then the page renders in German, the longest labels the layout check measures. */
  it("reads a shipped language", () => {
    expect(readHarnessLanguage("?page=squad&lang=de")).toBe("de");
    expect(readHarnessLanguage("?lang=pt-BR")).toBe("pt-BR");
  });

  /** Given no language, or one the game doesn't ship, then English. */
  it("falls back to English", () => {
    expect(readHarnessLanguage("?page=squad")).toBe("en");
    expect(readHarnessLanguage("?lang=klingon")).toBe("en");
  });
});

describe("readHarnessPage", () => {
  /** Given ?page=squad or ?page=home among other options, then that page is mounted. */
  it("reads a page the harness can mount", () => {
    expect(readHarnessPage("?theme=dark&page=squad")).toBe("squad");
    expect(readHarnessPage("?page=home")).toBe("home");
  });

  /** Given each world list's page, then the dashboard opens on that tab. */
  it("reads the world lists: players, teams, managers and staff", () => {
    expect(readHarnessPage("?page=players")).toBe("players");
    expect(readHarnessPage("?page=teams")).toBe("teams");
    expect(readHarnessPage("?page=managers")).toBe("managers");
    expect(readHarnessPage("?page=staff")).toBe("staff");
    expect(HARNESS_PAGES).toMatchObject({
      players: "Players",
      teams: "Teams",
      managers: "Managers",
      staff: "Staff",
    });
  });

  /** Given no page, or one the harness doesn't know, then the style guide itself renders. */
  it("leaves the style guide alone otherwise", () => {
    expect(readHarnessPage("?theme=dark")).toBeNull();
    expect(readHarnessPage("?page=tactics")).toBeNull();
  });
});
