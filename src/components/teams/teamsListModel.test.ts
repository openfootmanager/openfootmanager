import { describe, expect, it } from "vitest";

import { getPlayerOvr } from "../../lib/helpers";
import type {
  GameStateData,
  PlayerData,
  StandingData,
  TeamData,
} from "../../store/gameStore";
import {
  buildTeamRegionGroups,
  filterTeamRegionGroups,
} from "./teamsListModel";

const t = ((key: string, opts?: { defaultValue?: string }) => {
  if (key === "teams.otherClubs") return "Other clubs";
  return opts?.defaultValue ?? key;
}) as unknown as Parameters<typeof buildTeamRegionGroups>[1];

function team(id: string, name: string, country: string): TeamData {
  return {
    id,
    name,
    short_name: name.slice(0, 3).toUpperCase(),
    country,
    city: `${name} City`,
    stadium_name: "Ground",
    stadium_capacity: 20000,
    finance: 0,
    manager_id: null,
    reputation: 50,
    wage_budget: 0,
    transfer_budget: 0,
    season_income: 0,
    season_expenses: 0,
    formation: "4-4-2",
    play_style: "Balanced",
    training_focus: "General",
    training_intensity: "Balanced",
    training_schedule: "Balanced",
    founded_year: 1900,
    colors: { primary: "#000", secondary: "#fff" },
    starting_xi_ids: [],
    form: [],
    history: [],
  } as unknown as TeamData;
}

function standing(teamId: string, points: number): StandingData {
  return {
    team_id: teamId,
    played: 1,
    won: 0,
    drawn: 0,
    lost: 0,
    goals_for: points,
    goals_against: 0,
    points,
  } as StandingData;
}

function gameWith(overrides: Partial<GameStateData>): GameStateData {
  return {
    clock: { current_date: "2026-08-01T00:00:00Z", start_date: "2026-07-01T00:00:00Z" },
    manager: { team_id: "br-1" },
    teams: [],
    players: [],
    competitions: [],
    league: null,
    ...overrides,
  } as unknown as GameStateData;
}

describe("teamsListModel", () => {
  it("groups clubs by domestic league and region, ordered by standing", () => {
    const game = gameWith({
      teams: [
        team("br-1", "Alpha", "BR"),
        team("br-2", "Beta", "BR"),
        team("gb-1", "Gamma", "GB"),
      ],
      players: [
        { id: "p1", team_id: "br-1", market_value: 100, ...ovr(70) } as unknown as PlayerData,
        { id: "p2", team_id: "br-1", market_value: 200, ...ovr(80) } as unknown as PlayerData,
        { id: "p3", team_id: "gb-1", market_value: 50, ...ovr(60) } as unknown as PlayerData,
      ],
      competitions: [
        {
          id: "br-league",
          name: "Brazil First Division",
          kind: "League",
          scope: "Domestic",
          season: 1,
          region_id: "south-america",
          participant_ids: ["br-1", "br-2"],
          fixtures: [],
          standings: [standing("br-2", 6), standing("br-1", 3)],
        },
        {
          id: "gb-league",
          name: "England First Division",
          kind: "League",
          scope: "Domestic",
          season: 1,
          region_id: "europe",
          participant_ids: ["gb-1"],
          fixtures: [],
          standings: [standing("gb-1", 0)],
        },
      ] as unknown as GameStateData["competitions"],
    });

    const groups = buildTeamRegionGroups(game, t);

    // Regions sorted by label: Europe before South America.
    expect(groups.map((region) => region.id)).toEqual([
      "europe",
      "south-america",
    ]);

    const southAmerica = groups.find((r) => r.id === "south-america")!;
    expect(southAmerica.teamCount).toBe(2);
    const brLeague = southAmerica.leagues[0];
    expect(brLeague.name).toBe("Brazil First Division");
    // Beta (6 pts) ranks above Alpha (3 pts).
    expect(brLeague.teams.map((card) => card.team.id)).toEqual(["br-2", "br-1"]);
    expect(brLeague.teams[0].leaguePos).toBe(1);

    // Roster stats use the players-by-team index.
    const alpha = brLeague.teams.find((card) => card.team.id === "br-1")!;
    expect(alpha.rosterSize).toBe(2);
    expect(alpha.totalValue).toBe(300);
    const expectedAvg = Math.round(
      (getPlayerOvr({ ...ovr(70) } as unknown as PlayerData) +
        getPlayerOvr({ ...ovr(80) } as unknown as PlayerData)) /
        2,
    );
    expect(alpha.avgOvr).toBe(expectedAvg);
  });

  it("places clubs without a league into an Other clubs bucket", () => {
    const game = gameWith({
      teams: [team("gb-1", "Gamma", "GB")],
      competitions: [],
      league: null,
    });

    const groups = buildTeamRegionGroups(game, t);

    expect(groups).toHaveLength(1);
    expect(groups[0].id).toBe("europe");
    expect(groups[0].leagues[0].name).toBe("Other clubs");
    expect(groups[0].leagues[0].teams[0].team.id).toBe("gb-1");
  });

  it("filters by club name and city while preserving structure", () => {
    const game = gameWith({
      teams: [team("br-1", "Alpha", "BR"), team("br-2", "Beta", "BR")],
      competitions: [
        {
          id: "br-league",
          name: "Brazil First Division",
          kind: "League",
          scope: "Domestic",
          season: 1,
          region_id: "south-america",
          participant_ids: ["br-1", "br-2"],
          fixtures: [],
          standings: [],
        },
      ] as unknown as GameStateData["competitions"],
    });

    const groups = buildTeamRegionGroups(game, t);
    const filtered = filterTeamRegionGroups(groups, "beta");

    expect(filtered).toHaveLength(1);
    expect(filtered[0].leagues[0].teams.map((c) => c.team.id)).toEqual(["br-2"]);
    expect(filterTeamRegionGroups(groups, "zzz")).toHaveLength(0);
  });
});

function ovr(value: number) {
  return {
    attributes: {
      pace: value,
      stamina: value,
      strength: value,
      agility: value,
      passing: value,
      shooting: value,
      tackling: value,
      dribbling: value,
      defending: value,
      positioning: value,
      vision: value,
      decisions: value,
      composure: value,
      aggression: value,
      teamwork: value,
      leadership: value,
      handling: value,
      reflexes: value,
      aerial: value,
    },
    position: "Forward",
    natural_position: "Forward",
  };
}
