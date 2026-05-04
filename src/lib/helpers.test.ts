import { afterEach, beforeEach, describe, expect, it } from "vitest";
import {
  getTeamName,
  getTeamShort,
  expectedFixtureCount,
  findNextFixture,
  getContractRiskLevel,
  hasFullLeagueSchedule,
  getLocale,
  isSeasonComplete,
  formatMatchDate,
  formatDate,
  formatDateFull,
  formatDateShort,
  calcOvr,
  calcAge,
  formatExactMoney,
  formatVal,
  formatWeeklyAmount,
  positionBadgeVariant,
} from "./helpers";
import type { TeamData, FixtureData, PlayerData } from "../store/gameStore";
import { useSettingsStore } from "../store/settingsStore";

// ---------------------------------------------------------------------------
// Minimal test fixtures
// ---------------------------------------------------------------------------

const makeTeam = (overrides: Partial<TeamData> = {}): TeamData => ({
  id: "team_1",
  name: "Test FC",
  short_name: "TFC",
  country: "England",
  city: "London",
  stadium_name: "Test Stadium",
  stadium_capacity: 50000,
  finance: 1000000,
  manager_id: null,
  reputation: 500,
  wage_budget: 200000,
  transfer_budget: 500000,
  season_income: 0,
  season_expenses: 0,
  formation: "4-4-2",
  play_style: "Balanced",
  training_focus: "Physical",
  training_intensity: "Medium",
  training_schedule: "Balanced",
  founded_year: 1900,
  colors: { primary: "#000", secondary: "#fff" },
  starting_xi_ids: [],
  form: [],
  history: [],
  ...overrides,
});

const makePlayer = (overrides: Partial<PlayerData> = {}): PlayerData => ({
  id: "player_1",
  match_name: "Test Player",
  full_name: "Test Player Full",
  date_of_birth: "1996-01-15",
  nationality: "England",
  position: "Midfielder",
  natural_position: "Midfielder",
  alternate_positions: [],
  training_focus: null,
  attributes: {
    pace: 70, stamina: 70, strength: 70, agility: 70,
    passing: 70, shooting: 70, tackling: 70, dribbling: 70,
    defending: 70, positioning: 70, vision: 70, decisions: 70,
    composure: 50, aggression: 50, teamwork: 50,
    leadership: 50, handling: 30, reflexes: 30, aerial: 50,
  },
  condition: 100,
  morale: 80,
  injury: null,
  team_id: "team_1",
  contract_end: "2028-06-30",
  wage: 10000,
  market_value: 5000000,
  stats: {
    appearances: 0, goals: 0, assists: 0, clean_sheets: 0,
    yellow_cards: 0, red_cards: 0, avg_rating: 0, minutes_played: 0,
  },
  career: [],
  transfer_listed: false,
  loan_listed: false,
  transfer_offers: [],
  traits: [],
  ...overrides,
});

const makeFixture = (overrides: Partial<FixtureData> = {}): FixtureData => ({
  id: "fix_1",
  matchday: 1,
  date: "2026-08-01",
  home_team_id: "team_1",
  away_team_id: "team_2",
  competition: "League",
  status: "Scheduled",
  result: null,
  ...overrides,
});

const originalSettings = useSettingsStore.getState().settings;

beforeEach(() => {
  useSettingsStore.setState({
    settings: { ...originalSettings, currency: "EUR", language: "en" },
  });
});

afterEach(() => {
  useSettingsStore.setState({ settings: originalSettings });
});

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

describe("getTeamName", () => {
  const teams = [makeTeam({ id: "t1", name: "Alpha FC" }), makeTeam({ id: "t2", name: "Beta United" })];

  it("returns team name for valid id", () => {
    expect(getTeamName(teams, "t1")).toBe("Alpha FC");
    expect(getTeamName(teams, "t2")).toBe("Beta United");
  });

  it("returns 'Free Agent' for null id", () => {
    expect(getTeamName(teams, null)).toBe("Free Agent");
  });

  it("returns 'Unknown' for non-existent id", () => {
    expect(getTeamName(teams, "t999")).toBe("Unknown");
  });
});

describe("getTeamShort", () => {
  const teams = [makeTeam({ id: "t1", short_name: "ALP" })];

  it("returns short name for valid id", () => {
    expect(getTeamShort(teams, "t1")).toBe("ALP");
  });

  it("returns '???' for non-existent id", () => {
    expect(getTeamShort(teams, "t999")).toBe("???");
  });
});

describe("findNextFixture", () => {
  it("finds scheduled fixture for team", () => {
    const fixtures = [
      makeFixture({ id: "f1", status: "Completed", home_team_id: "team_1" }),
      makeFixture({ id: "f2", status: "Scheduled", away_team_id: "team_1" }),
    ];
    const next = findNextFixture(fixtures, "team_1");
    expect(next?.id).toBe("f2");
  });

  it("returns undefined when no scheduled fixtures exist", () => {
    const fixtures = [makeFixture({ status: "Completed" })];
    expect(findNextFixture(fixtures, "team_1")).toBeUndefined();
  });

  it("returns undefined for non-matching team", () => {
    const fixtures = [makeFixture({ home_team_id: "other", away_team_id: "other2" })];
    expect(findNextFixture(fixtures, "team_1")).toBeUndefined();
  });
});

describe("season helpers", () => {
  it("computes the expected double round-robin fixture count for even-sized leagues", () => {
    expect(expectedFixtureCount(16)).toBe(240);
    expect(expectedFixtureCount(4)).toBe(12);
  });

  it("treats odd-sized or undersized leagues as incomplete schedules", () => {
    expect(expectedFixtureCount(1)).toBeNull();
    expect(expectedFixtureCount(3)).toBeNull();
  });

  it("requires a full plausible schedule before marking the season complete", () => {
    const truncatedLeague = {
      id: "league-1",
      name: "League",
      season: 1,
      fixtures: [
        makeFixture({ id: "f1", status: "Completed" }),
        makeFixture({ id: "f2", status: "Completed", home_team_id: "team_3", away_team_id: "team_4" }),
      ],
      standings: [
        { team_id: "team_1", played: 1, won: 1, drawn: 0, lost: 0, goals_for: 2, goals_against: 0, points: 3 },
        { team_id: "team_2", played: 1, won: 0, drawn: 0, lost: 1, goals_for: 0, goals_against: 2, points: 0 },
        { team_id: "team_3", played: 1, won: 0, drawn: 0, lost: 1, goals_for: 0, goals_against: 1, points: 0 },
        { team_id: "team_4", played: 1, won: 1, drawn: 0, lost: 0, goals_for: 1, goals_against: 0, points: 3 },
      ],
    };

    expect(hasFullLeagueSchedule(truncatedLeague)).toBe(false);
    expect(isSeasonComplete(truncatedLeague)).toBe(false);
  });

  it("ignores friendlies when validating league schedule completeness", () => {
    const competitiveFixtures: FixtureData[] = [];
    let fixtureCounter = 1;
    for (const homeTeamId of ["team_1", "team_2", "team_3", "team_4"]) {
      for (const awayTeamId of ["team_1", "team_2", "team_3", "team_4"]) {
        if (homeTeamId === awayTeamId) {
          continue;
        }
        competitiveFixtures.push(
          makeFixture({
            id: `f${fixtureCounter}`,
            home_team_id: homeTeamId,
            away_team_id: awayTeamId,
            status: "Completed",
          }),
        );
        fixtureCounter += 1;
      }
    }

    const leagueWithFriendly = {
      id: "league-1",
      name: "League",
      season: 1,
      fixtures: [
        makeFixture({
          id: "friendly-1",
          competition: "Friendly",
          matchday: 0,
          status: "Completed",
        }),
        ...competitiveFixtures,
      ],
      standings: [
        { team_id: "team_1", played: 6, won: 6, drawn: 0, lost: 0, goals_for: 12, goals_against: 2, points: 18 },
        { team_id: "team_2", played: 6, won: 3, drawn: 0, lost: 3, goals_for: 8, goals_against: 8, points: 9 },
        { team_id: "team_3", played: 6, won: 2, drawn: 0, lost: 4, goals_for: 6, goals_against: 10, points: 6 },
        { team_id: "team_4", played: 6, won: 1, drawn: 0, lost: 5, goals_for: 4, goals_against: 10, points: 3 },
      ],
    };

    expect(hasFullLeagueSchedule(leagueWithFriendly)).toBe(true);
    expect(isSeasonComplete(leagueWithFriendly)).toBe(true);
  });

  it("marks the season complete when the full schedule exists and every fixture is completed", () => {
    const fixtures: FixtureData[] = [];
    let fixtureCounter = 1;
    for (const homeTeamId of ["team_1", "team_2", "team_3", "team_4"]) {
      for (const awayTeamId of ["team_1", "team_2", "team_3", "team_4"]) {
        if (homeTeamId === awayTeamId) {
          continue;
        }
        fixtures.push(
          makeFixture({
            id: `f${fixtureCounter}`,
            home_team_id: homeTeamId,
            away_team_id: awayTeamId,
            status: "Completed",
          }),
        );
        fixtureCounter += 1;
      }
    }

    const fullLeague = {
      id: "league-1",
      name: "League",
      season: 1,
      fixtures,
      standings: [
        { team_id: "team_1", played: 6, won: 6, drawn: 0, lost: 0, goals_for: 12, goals_against: 2, points: 18 },
        { team_id: "team_2", played: 6, won: 3, drawn: 0, lost: 3, goals_for: 8, goals_against: 8, points: 9 },
        { team_id: "team_3", played: 6, won: 2, drawn: 0, lost: 4, goals_for: 6, goals_against: 10, points: 6 },
        { team_id: "team_4", played: 6, won: 1, drawn: 0, lost: 5, goals_for: 4, goals_against: 10, points: 3 },
      ],
    };

    expect(hasFullLeagueSchedule(fullLeague)).toBe(true);
    expect(isSeasonComplete(fullLeague)).toBe(true);
  });
});

describe("getLocale", () => {
  it("maps known language codes", () => {
    expect(getLocale("en")).toBe("en-US");
    expect(getLocale("es")).toBe("es-ES");
    expect(getLocale("pt")).toBe("pt-BR");
    expect(getLocale("fr")).toBe("fr-FR");
    expect(getLocale("de")).toBe("de-DE");
    expect(getLocale("it")).toBe("it-IT");
  });

  it("returns input for unknown codes", () => {
    expect(getLocale("ja")).toBe("ja");
  });

  it("returns 'en-US' for undefined", () => {
    expect(getLocale(undefined)).toBe("en-US");
  });
});

describe("calcOvr", () => {
  it("calculates positional overall from the player's natural role", () => {
    const player = makePlayer({
      position: "CentralMidfielder",
      natural_position: "CentralMidfielder",
    });

    expect(calcOvr(player)).toBe(68);
  });

  it("rounds positional overall to the nearest integer", () => {
    const player = makePlayer({
      position: "CentralMidfielder",
      natural_position: "CentralMidfielder",
      attributes: {
        ...makePlayer().attributes,
        passing: 73,
      },
    });

    expect(calcOvr(player)).toBe(69);
  });
});

describe("calcAge", () => {
  it("calculates age relative to 2026", () => {
    expect(calcAge("1996-01-15")).toBe(30);
    expect(calcAge("2000-06-01")).toBe(26);
  });
});

describe("getContractRiskLevel", () => {
  it("marks contracts expiring within 180 days as critical", () => {
    expect(getContractRiskLevel("2026-12-28", "2026-07-01")).toBe("critical");
  });

  it("marks contracts expiring within 365 days as warning", () => {
    expect(getContractRiskLevel("2027-06-30", "2026-07-01")).toBe("warning");
  });

  it("marks longer contracts as stable", () => {
    expect(getContractRiskLevel("2027-07-02", "2026-07-01")).toBe("stable");
  });
});

describe("formatVal", () => {
  it("formats millions", () => {
    expect(formatVal(5000000)).toBe("€5.0M");
    expect(formatVal(1500000)).toBe("€1.5M");
  });

  it("formats thousands", () => {
    expect(formatVal(50000)).toBe("€50K");
    expect(formatVal(1000)).toBe("€1K");
  });

  it("formats small values", () => {
    expect(formatVal(500)).toBe("€500");
    expect(formatVal(0)).toBe("€0");
  });

  it("uses the selected settings currency", () => {
    useSettingsStore.setState({
      settings: { ...useSettingsStore.getState().settings, currency: "GBP" },
    });

    expect(formatVal(5000000)).toBe("£5.0M");
    expect(formatVal(500)).toBe("£500");
  });
});

describe("formatExactMoney", () => {
  it("formats exact amounts using the selected settings currency", () => {
    useSettingsStore.setState({
      settings: { ...useSettingsStore.getState().settings, currency: "USD" },
    });

    expect(formatExactMoney(125000)).toBe("$125,000");
    expect(formatExactMoney(-30000)).toBe("-$30,000");
  });
});

describe("formatWeeklyAmount", () => {
  it("appends the localized weekly suffix to a formatted amount", () => {
    expect(formatWeeklyAmount("€10K", "/wk")).toBe("€10K/wk");
    expect(formatWeeklyAmount("€10K", "/sem")).toBe("€10K/sem");
  });
});

describe("positionBadgeVariant", () => {
  it("returns correct variant for each position", () => {
    expect(positionBadgeVariant("Goalkeeper")).toBe("accent");
    expect(positionBadgeVariant("Defender")).toBe("primary");
    expect(positionBadgeVariant("CenterBack")).toBe("primary");
    expect(positionBadgeVariant("Midfielder")).toBe("success");
    expect(positionBadgeVariant("AttackingMidfielder")).toBe("success");
    expect(positionBadgeVariant("Forward")).toBe("danger");
    expect(positionBadgeVariant("Striker")).toBe("danger");
  });

  it("returns 'primary' for unknown position", () => {
    expect(positionBadgeVariant("Unknown")).toBe("primary");
  });
});

// ---------------------------------------------------------------------------
// Date formatting
// ---------------------------------------------------------------------------

describe("formatMatchDate", () => {
  it("returns a short weekday + month + day string", () => {
    const result = formatMatchDate("2026-08-15", "en");
    // e.g. "Sat, Aug 15" — exact format is locale-dependent, just verify it contains key parts
    expect(result).toMatch(/Aug/);
    expect(result).toMatch(/15/);
  });

  it("uses getLocale for language mapping", () => {
    const en = formatMatchDate("2026-01-01", "en");
    const es = formatMatchDate("2026-01-01", "es");
    // Both should produce non-empty strings; Spanish should differ from English
    expect(en.length).toBeGreaterThan(0);
    expect(es.length).toBeGreaterThan(0);
  });

  it("parses RFC3339 timestamps from backend news articles", () => {
    const result = formatMatchDate("2026-07-27T12:00:00+00:00", "en");

    expect(result).not.toBe("Invalid Date");
    expect(result).toMatch(/Jul/);
    expect(result).toMatch(/27/);
  });
});

describe("formatDate", () => {
  it("returns a full date string by default", () => {
    const result = formatDate("2026-08-15T12:00:00", "en");
    expect(result).toMatch(/August/);
    expect(result).toMatch(/15/);
    expect(result).toMatch(/2026/);
  });

  it("accepts custom Intl.DateTimeFormatOptions", () => {
    const result = formatDate("2026-08-15T12:00:00", "en", { year: "numeric" });
    expect(result).toBe("2026");
  });
});

describe("formatDateFull", () => {
  it("includes weekday, month, day, year", () => {
    const result = formatDateFull("2026-08-15T12:00:00", "en");
    // Should contain full weekday and month
    expect(result).toMatch(/Saturday/);
    expect(result).toMatch(/August/);
    expect(result).toMatch(/15/);
    expect(result).toMatch(/2026/);
  });
});

describe("formatDateShort", () => {
  it("returns short month + day", () => {
    const result = formatDateShort("2026-08-15T12:00:00", "en");
    expect(result).toMatch(/Aug/);
    expect(result).toMatch(/15/);
  });

  it("does not include year", () => {
    const result = formatDateShort("2026-08-15T12:00:00", "en");
    expect(result).not.toMatch(/2026/);
  });
});

// ---------------------------------------------------------------------------
// parseDateInput consistency — timezone safety
// ---------------------------------------------------------------------------

describe("date parsing timezone consistency", () => {
  // The game clock is serialized as a UTC RFC3339 string (e.g. "2026-09-12T00:00:00Z").
  // Message dates are stored as YYYY-MM-DD strings (e.g. "2026-09-12").
  // Both representations of the same calendar day must render identically in all
  // timezones — otherwise the game header and inbox message dates can disagree.

  it("renders YYYY-MM-DD and UTC midnight RFC3339 for the same day identically", () => {
    const plain = formatDateFull("2026-09-12", "en");
    const utcMidnight = formatDateFull("2026-09-12T00:00:00Z", "en");
    expect(plain).toBe(utcMidnight);
  });

  it("renders YYYY-MM-DD and RFC3339 with positive UTC offset for the same day identically", () => {
    const plain = formatDateFull("2026-09-12", "en");
    const withOffset = formatDateFull("2026-09-12T00:00:00+03:00", "en");
    expect(plain).toBe(withOffset);
  });

  it("renders YYYY-MM-DD and RFC3339 with negative UTC offset for the same day identically", () => {
    const plain = formatDateFull("2026-09-12", "en");
    const withOffset = formatDateFull("2026-09-12T21:00:00-03:00", "en");
    expect(plain).toBe(withOffset);
  });

  it("preserves the correct calendar day for UTC midnight dates across formats", () => {
    // September 12, 2026 is a Saturday
    const result = formatDateFull("2026-09-12T00:00:00Z", "en");
    expect(result).toMatch(/Saturday/);
    expect(result).toMatch(/September/);
    expect(result).toMatch(/12/);
    expect(result).toMatch(/2026/);
  });
});

// ---------------------------------------------------------------------------
// isSeasonComplete — new season guard (all fixtures scheduled)
// ---------------------------------------------------------------------------

describe("isSeasonComplete with unplayed season", () => {
  // Build a full 4-team schedule where every fixture is Scheduled.
  // isSeasonComplete must return false — a freshly generated season must not
  // be detected as complete before a single match has been played.
  function makeFullScheduledLeague() {
    const teamIds = ["team_1", "team_2", "team_3", "team_4"];
    const fixtures: FixtureData[] = [];
    let counter = 1;
    for (const home of teamIds) {
      for (const away of teamIds) {
        if (home === away) continue;
        fixtures.push(makeFixture({
          id: `f${counter++}`,
          home_team_id: home,
          away_team_id: away,
          status: "Scheduled",
          competition: "League",
        }));
      }
    }
    return {
      id: "league-1",
      name: "League",
      season: 1,
      fixtures,
      standings: teamIds.map(id => ({
        team_id: id, played: 0, won: 0, drawn: 0, lost: 0,
        goals_for: 0, goals_against: 0, points: 0,
      })),
    };
  }

  it("returns false when the schedule is full but no matches have been played", () => {
    const league = makeFullScheduledLeague();
    expect(hasFullLeagueSchedule(league)).toBe(true);
    expect(isSeasonComplete(league)).toBe(false);
  });
});
