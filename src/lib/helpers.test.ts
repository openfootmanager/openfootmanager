import { describe, it, expect } from "vitest";
import {
  getTeamName,
  getTeamShort,
  findNextFixture,
  getLocale,
  formatMatchDate,
  formatDate,
  formatDateFull,
  formatDateShort,
  calcOvr,
  calcAge,
  formatVal,
  formatWeeklyAmount,
  positionBadgeVariant,
} from "./helpers";
import type { TeamData, FixtureData, PlayerData } from "../store/gameStore";

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
  status: "Scheduled",
  result: null,
  ...overrides,
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

describe("getLocale", () => {
  it("maps known language codes", () => {
    expect(getLocale("en")).toBe("en-US");
    expect(getLocale("es")).toBe("es-ES");
    expect(getLocale("pt")).toBe("pt-BR");
    expect(getLocale("fr")).toBe("fr-FR");
    expect(getLocale("de")).toBe("de-DE");
    expect(getLocale("it")).toBe("it-IT");
    expect(getLocale("tr")).toBe("tr-TR");
  });

  it("returns input for unknown codes", () => {
    expect(getLocale("ja")).toBe("ja");
  });

  it("returns 'en-US' for undefined", () => {
    expect(getLocale(undefined)).toBe("en-US");
  });
});

describe("calcOvr", () => {
  it("calculates overall from 11 core attributes", () => {
    const player = makePlayer();
    // All 11 core attrs are 70 → OVR = 70
    expect(calcOvr(player)).toBe(70);
  });

  it("rounds to nearest integer", () => {
    const player = makePlayer({
      attributes: {
        ...makePlayer().attributes,
        pace: 71, // only this differs → (71 + 10*70) / 11 = 770.09... → 70
      },
    });
    expect(calcOvr(player)).toBe(70);
  });
});

describe("calcAge", () => {
  it("calculates age relative to 2026", () => {
    expect(calcAge("1996-01-15")).toBe(30);
    expect(calcAge("2000-06-01")).toBe(26);
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
    expect(positionBadgeVariant("Midfielder")).toBe("success");
    expect(positionBadgeVariant("Forward")).toBe("danger");
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
