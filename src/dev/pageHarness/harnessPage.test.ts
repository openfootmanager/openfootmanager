import { describe, expect, it } from "vitest";

import {
  HARNESS_PAGES,
  HARNESS_PROFILES,
  readHarnessLanguage,
  readHarnessPage,
  readHarnessProfile,
} from "./harnessPage";

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

describe("readHarnessPage: the player profile", () => {
  /** Given ?page=profile, then the dashboard opens on a player's profile (under the Players tab). */
  it("reads the profile page", () => {
    expect(readHarnessPage("?page=profile&player=rival")).toBe("profile");
    expect(HARNESS_PAGES.profile).toBe("Players");
  });
});

describe("readHarnessProfile", () => {
  /** Given each of the five knowledge states the club can have of a player, then that one is read. */
  it("reads each state the profile can be in", () => {
    expect(HARNESS_PROFILES).toEqual([
      "own",
      "keeper",
      "injured",
      "loaned",
      "rival",
      "scouted",
      "unknown",
      "retired",
    ]);
    for (const player of HARNESS_PROFILES) {
      expect(readHarnessProfile(`?page=profile&player=${player}`)).toBe(player);
    }
  });

  /** Given no player, or one the harness doesn't know, then the club's own player. */
  it("falls back to the club's own player", () => {
    expect(readHarnessProfile("?page=profile")).toBe("own");
    expect(readHarnessProfile("?page=profile&player=nobody")).toBe("own");
  });
});
