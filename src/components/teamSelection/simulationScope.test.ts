import { describe, expect, it } from "vitest";

import type { LeagueData, TeamData, WorldRegionData } from "../../store/gameStore";
import { createTeam } from "../../test-utils/factories";
import {
  activeRegionIds,
  clubsInRegions,
  competitionRequiredRegions,
  competitionState,
  enabledCompetitionIds,
  groupCompetitions,
  lockedRegions,
  type ScopeDraft,
} from "./simulationScope";

function competition(overrides: Partial<LeagueData>): LeagueData {
  return {
    id: "c",
    name: "C",
    season: 2026,
    fixtures: [],
    standings: [],
    ...overrides,
  } as LeagueData;
}

const league = competition;

const REGIONS: WorldRegionData[] = [
  { id: "scandinavia", name: "Scandinavia", country_codes: ["SE", "NO"] },
  { id: "western", name: "Western Europe", country_codes: ["DE", "FR"] },
];

const DRAFT: ScopeDraft = {
  homeRegionId: "scandinavia",
  homeCountryCode: "SE",
  regions: { western: false },
  competitions: {},
};

describe("competitionRequiredRegions", () => {
  it("adds the region_id for Domestic and Regional scopes", () => {
    expect(competitionRequiredRegions(league({ scope: "Domestic", region_id: "europe" }))).toEqual([
      "europe",
    ]);
    expect(competitionRequiredRegions(league({ scope: "Regional", region_id: "asia" }))).toEqual([
      "asia",
    ]);
  });

  it("does not add the region_id for Continental/International scopes", () => {
    expect(
      competitionRequiredRegions(league({ scope: "Continental", region_id: "europe" })),
    ).toEqual([]);
  });

  it("merges and de-duplicates explicit required_region_ids with the scope region, sorted", () => {
    expect(
      competitionRequiredRegions(
        league({
          scope: "Domestic",
          region_id: "europe",
          required_region_ids: ["south_america", "europe"],
        }),
      ),
    ).toEqual(["europe", "south_america"]);
  });
});

describe("lockedRegions", () => {
  /**
   * Given a home region and a chosen club whose continental cup needs a second region
   * When the locked regions are worked out
   * Then the home region is locked as home and the cup's region as the club's; with no club chosen
   *      only the home region is locked.
   */
  it("locks the home region, and the regions the chosen club's competitions need", () => {
    const cup = competition({ id: "cup", scope: "Continental", required_region_ids: ["western"] });

    expect([...lockedRegions("scandinavia", [cup])]).toEqual([
      ["western", "yourClub"],
      ["scandinavia", "home"],
    ]);
    expect([...lockedRegions("scandinavia", [])]).toEqual([["scandinavia", "home"]]);
  });
});

describe("activeRegionIds", () => {
  /**
   * Given a home region, a region switched on, and a switched-off one
   * When the regions simulated are worked out
   * Then the home region and the switched-on one are in, sorted; and a region no club is in (a
   *      competition's own) is simulated once it is switched on.
   */
  it("is the locked regions and the switched-on ones, whether or not the world lists clubs there", () => {
    const draft: ScopeDraft = { ...DRAFT, regions: { western: true, asia: true, southern: false } };

    expect(activeRegionIds(draft, lockedRegions("scandinavia", []))).toEqual([
      "asia",
      "scandinavia",
      "western",
    ]);
  });
});

describe("competitionState", () => {
  const de1 = competition({ id: "de1", scope: "Domestic", region_id: "western" });
  const locks = lockedRegions("scandinavia", []);

  /**
   * Given a competition in a region that is off
   * When its state is worked out, and then the region is on
   * Then it is blocked and not simulated, whatever was chosen, naming the region it needs; with the
   *      region on, its remembered choice decides.
   */
  it("blocks a competition whose region is off, and lets the remembered choice decide once it is on", () => {
    const chosen: ScopeDraft = { ...DRAFT, competitions: { de1: true } };

    const off = competitionState(de1, chosen, new Set(), new Set(activeRegionIds(chosen, locks)));
    expect(off).toMatchObject({ on: false, blocked: true, missingRegionIds: ["western"] });

    const regionOn: ScopeDraft = { ...chosen, regions: { western: true } };
    const on = competitionState(
      de1,
      regionOn,
      new Set(),
      new Set(activeRegionIds(regionOn, locks)),
    );
    expect(on).toMatchObject({ on: true, blocked: false, missingRegionIds: [] });

    const notChosen = competitionState(
      de1,
      { ...regionOn, competitions: { de1: false } },
      new Set(),
      new Set(["scandinavia", "western"]),
    );
    expect(notChosen.on).toBe(false);
  });

  it("always simulates the chosen club's competition, whatever the draft says", () => {
    const se1 = competition({ id: "se1", scope: "Domestic", region_id: "scandinavia" });

    const state = competitionState(
      se1,
      { ...DRAFT, competitions: { se1: false } },
      new Set(["se1"]),
      new Set(["scandinavia"]),
    );

    expect(state).toMatchObject({ on: true, yours: true });
  });
});

describe("enabledCompetitionIds", () => {
  /**
   * Given three competitions: one at home and chosen, one in a region that is off, one at home and
   *       not chosen
   * When the ids to simulate are worked out
   * Then only the first: a blocked or unchosen competition is never sent on to the game.
   */
  it("leaves out a blocked competition and an unchosen one", () => {
    const se1 = competition({ id: "se1", scope: "Domestic", region_id: "scandinavia" });
    const de1 = competition({ id: "de1", scope: "Domestic", region_id: "western" });
    const se3 = competition({ id: "se3", scope: "Domestic", region_id: "scandinavia" });
    const draft: ScopeDraft = { ...DRAFT, competitions: { se1: true, de1: true, se3: false } };

    expect(
      enabledCompetitionIds([se1, de1, se3], draft, new Set(), new Set(["scandinavia"])),
    ).toEqual(["se1"]);
  });
});

describe("groupCompetitions", () => {
  it("groups by scope, domestic first, and leaves empty groups out", () => {
    const groups = groupCompetitions([
      competition({ id: "wc", scope: "International" }),
      competition({ id: "se1", scope: "Domestic" }),
      competition({ id: "cc", scope: "Continental" }),
      competition({ id: "se2", scope: "Domestic" }),
    ]);

    expect(groups.map((group) => [group.scope, group.competitions.map((c) => c.id)])).toEqual([
      ["Domestic", ["se1", "se2"]],
      ["Continental", ["cc"]],
      ["International", ["wc"]],
    ]);
  });
});

describe("clubsInRegions", () => {
  it("counts the clubs whose country is in a region simulated", () => {
    const teams: TeamData[] = [
      createTeam({ id: "a", country: "SE" }),
      createTeam({ id: "b", country: "NO" }),
      createTeam({ id: "c", country: "DE" }),
    ];

    expect(clubsInRegions(REGIONS, new Set(["scandinavia"]), teams)).toBe(2);
    expect(clubsInRegions(REGIONS, new Set(["scandinavia", "western"]), teams)).toBe(3);
  });
});
