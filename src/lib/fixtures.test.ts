import { afterAll, beforeAll, describe, expect, it } from "vitest";

import i18n, { i18nReady } from "../i18n";
import type { FixtureCompetition } from "../store/types";
import type { LeagueData } from "../store/types";
import { getFixtureCompetitionName, getFixtureDisplayLabel, getUserCompetition } from "./fixtures";

let previousLanguage: string;

beforeAll(async () => {
  await i18nReady;
  previousLanguage = i18n.language;
  await i18n.changeLanguage("en");
  await i18n.loadLanguages("pt-BR");
});

afterAll(async () => {
  await i18n.changeLanguage(previousLanguage);
});

function label(competition: FixtureCompetition, competitionName?: string): string {
  return getFixtureDisplayLabel(i18n.t, { competition, matchday: 3 }, competitionName);
}

describe("getFixtureDisplayLabel", () => {
  it("labels a league fixture with its matchday", () => {
    expect(label("League", "Premier Division")).toBe("Matchday 3");
  });

  it("labels friendlies and pre-season tournaments by what they are", () => {
    expect(label("Friendly", "Premier Division")).toBe("Friendly");
    expect(label("PreseasonTournament")).toBe("Preseason Tournament");
  });

  it.each<[FixtureCompetition, string]>([
    ["Cup", "Cup"],
    ["ContinentalClub", "Continental Club"],
    ["InternationalClub", "International Club"],
    ["InternationalNation", "International Nation"],
    ["FriendlyCup", "Friendly Cup"],
  ])("labels a %s fixture by its competition kind when the name is unknown", (kind, expected) => {
    expect(label(kind)).toBe(expected);
  });

  it.each<FixtureCompetition>([
    "Cup",
    "ContinentalClub",
    "InternationalClub",
    "InternationalNation",
    "FriendlyCup",
  ])("labels a %s fixture with its competition name when one is given", (kind) => {
    expect(label(kind, "Copa Libertadores")).toBe("Copa Libertadores");
  });
});

describe("getFixtureCompetitionName", () => {
  it("translates a named World Cup in the requested locale", () => {
    const gameState = {
      competitions: [
        {
          id: "world-cup-2026",
          name: "2026 World Cup",
          name_key: "tournaments.competitions.worldCup",
          season: 2026,
          fixtures: [],
          standings: [],
        },
      ],
    };

    expect(
      getFixtureCompetitionName(
        gameState,
        { competition_id: "world-cup-2026" },
        i18n.getFixedT("pt-BR"),
      ),
    ).toBe("Copa do Mundo 2026");
  });
});

describe("getUserCompetition", () => {
  const mirror = { id: "league-1", name: "First Division" } as LeagueData;
  const stint = { team_id: "t1", team_name: "Alpha FC" } as never;
  const game = (team_id: string | null, career_history: never[]) => ({
    manager: { team_id, career_history },
    competitions: [],
    league: mirror,
  });

  /**
   * Given a manager sacked from a club (no club now, one in his history), and the mirror the game
   *       keeps on his last club's league
   * When the screens ask for his competition
   * Then they get it: a sacked manager reads the league he left
   */
  it("gives a sacked manager the mirror, which follows his last club", () => {
    expect(getUserCompetition(game(null, [stint]))).toBe(mirror);
  });

  /**
   * Given a manager who has never had a club, and the mirror on the world's first competition
   *       (kept there for the season's rollover)
   * When the screens ask for his competition
   * Then they get nothing: the Home screens show him none of a league he has nothing to do with
   */
  it("gives a manager who never had a club no competition", () => {
    expect(getUserCompetition(game(null, []))).toBeNull();
  });

  /**
   * Given an employed manager in a save with only the legacy mirror
   * When the screens ask for his competition
   * Then they get the mirror, as before
   */
  it("keeps the legacy mirror for an employed manager of an old save", () => {
    expect(getUserCompetition(game("t1", []))).toBe(mirror);
  });
});
