import i18n, { i18nReady } from "../i18n";
import { beforeAll, describe, expect, it } from "vitest";

import { competitionDisplayName, localizeCompetitionName } from "./competitionName";

const t = (key: string, options?: Record<string, unknown>) => i18n.t(key, options) as string;

const germanFirst = {
  name: "Germany First Division",
  name_key: "tournaments.competitions.firstDivision",
  country_id: "DE",
  season: 2027,
};

beforeAll(async () => {
  await i18nReady;
  await i18n.changeLanguage("de");
});

describe("localizeCompetitionName", () => {
  /**
   * Given a stored English template name ("Germany First Division") and the game's competitions,
   *       one of which carries that name with its name key and country
   * When the name is shown in German
   * Then it reads as every other screen names it, "Deutschland Erste Division".
   */
  it("names a stored template name through its competition's name key", () => {
    expect(localizeCompetitionName("Germany First Division", [germanFirst], t)).toBe(
      competitionDisplayName(germanFirst, t),
    );
    expect(localizeCompetitionName("Germany First Division", [germanFirst], t)).toBe(
      "Deutschland Erste Division",
    );
  });

  /**
   * Given a name no competition carries a key for (a hand-made league, or none loaded)
   * When it is shown
   * Then the stored name stands.
   */
  it("keeps a name that has no competition with a key", () => {
    expect(localizeCompetitionName("Castle League", [germanFirst], t)).toBe("Castle League");
    expect(localizeCompetitionName("Castle League", undefined, t)).toBe("Castle League");
    expect(
      localizeCompetitionName("Castle League", [{ name: "Castle League", name_key: null }], t),
    ).toBe("Castle League");
  });

  /**
   * Given the same name on several editions of one competition
   * When it is shown
   * Then the latest edition's season is the one interpolated.
   */
  it("uses the latest edition when several carry the name", () => {
    const worldCup = (season: number) => ({
      name: "World Cup",
      name_key: "tournaments.competitions.worldCup",
      season,
    });

    expect(localizeCompetitionName("World Cup", [worldCup(2022), worldCup(2026)], t)).toContain(
      "2026",
    );
  });
});
