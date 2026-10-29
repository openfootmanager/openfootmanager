import { afterEach, beforeEach, describe, expect, it } from "vitest";
import {
  getTeamName,
  getTeamShort,
  findNextFixture,
  formatMatchDate,
  formatDate,
  formatDateFull,
  formatDateShort,
  calcAge,
  formatExactMoney,
  formatVal,
  formatWeeklyAmount,
  getPlayerOvr,
} from "./helpers";
import type { TeamData, FixtureData, PlayerData } from "../store/gameStore";
import { useGameStore, type GameStateData } from "../store/gameStore";
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
    pace: 70,
    stamina: 70,
    strength: 70,
    agility: 70,
    passing: 70,
    shooting: 70,
    tackling: 70,
    dribbling: 70,
    defending: 70,
    positioning: 70,
    vision: 70,
    decisions: 70,
    composure: 50,
    aggression: 50,
    teamwork: 50,
    leadership: 50,
    handling: 30,
    reflexes: 30,
    aerial: 50,
  },
  condition: 100,
  morale: 80,
  injury: null,
  team_id: "team_1",
  retired: false,
  contract_end: "2028-06-30",
  wage: 10000,
  market_value: 5000000,
  stats: {
    appearances: 0,
    goals: 0,
    assists: 0,
    clean_sheets: 0,
    yellow_cards: 0,
    red_cards: 0,
    avg_rating: 0,
    minutes_played: 0,
  },
  career: [],
  transfer_listed: false,
  loan_listed: false,
  transfer_offers: [],
  traits: [],
  ...overrides,
});

const makeFixture = (overrides: Partial<FixtureData> = {}): FixtureData => ({
  competitive: true,
  advancing_team_id: null,
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
const originalCurrency = useSettingsStore.getState().currency;
const originalSupportedCurrencies = useSettingsStore.getState().supportedCurrencies;
const SUPPORTED_CURRENCIES = {
  EUR: { code: "EUR", symbol: "€", exchange_rate: 1 },
  GBP: { code: "GBP", symbol: "£", exchange_rate: 0.86 },
  USD: { code: "USD", symbol: "$", exchange_rate: 1.08 },
} as const;

beforeEach(() => {
  useSettingsStore.setState({
    settings: { ...originalSettings, currency: "EUR", language: "en" },
    currency: SUPPORTED_CURRENCIES.EUR,
    supportedCurrencies: SUPPORTED_CURRENCIES,
  });
});

afterEach(() => {
  useSettingsStore.setState({
    settings: originalSettings,
    currency: originalCurrency,
    supportedCurrencies: originalSupportedCurrencies,
  });
});

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

describe("getTeamName", () => {
  const teams = [
    makeTeam({ id: "t1", name: "Alpha FC" }),
    makeTeam({ id: "t2", name: "Beta United" }),
  ];

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

  it("returns the earliest scheduled fixture by date instead of array order", () => {
    const fixtures = [
      makeFixture({
        id: "f2",
        date: "2026-07-24",
        home_team_id: "team_2",
        away_team_id: "team_1",
      }),
      makeFixture({
        id: "f1",
        date: "2026-07-17",
        home_team_id: "team_3",
        away_team_id: "team_1",
      }),
    ];

    expect(findNextFixture(fixtures, "team_1")?.id).toBe("f1");
  });
});

describe("getPlayerOvr", () => {
  it("returns the backend ovr when present", () => {
    expect(getPlayerOvr(makePlayer({ ovr: 68 }))).toBe(68);
  });

  it("falls back to 0 when backend ovr is missing", () => {
    expect(getPlayerOvr(makePlayer({ ovr: undefined }))).toBe(0);
  });
});

describe("calcAge", () => {
  // Regression: calcAge was hardcoded to `2026 - birth year`, so every
  // displayed age froze at the starting season as the game clock advanced.
  afterEach(() => {
    useGameStore.getState().clearGame();
  });

  it("ages players as the in-game clock advances across seasons", () => {
    useGameStore.setState({
      gameState: { clock: { current_date: "2026-08-01" } } as unknown as GameStateData,
    });
    expect(calcAge("1996-01-15")).toBe(30);

    useGameStore.setState({
      gameState: { clock: { current_date: "2028-08-01" } } as unknown as GameStateData,
    });
    expect(calcAge("1996-01-15")).toBe(32);
  });

  it("only increments after the birthday has passed", () => {
    useGameStore.setState({
      gameState: { clock: { current_date: "2027-05-01" } } as unknown as GameStateData,
    });
    expect(calcAge("2000-06-01")).toBe(26);

    useGameStore.setState({
      gameState: { clock: { current_date: "2027-06-02" } } as unknown as GameStateData,
    });
    expect(calcAge("2000-06-01")).toBe(27);
  });
});

describe("formatVal", () => {
  it("formats millions", () => {
    expect(formatVal(5000000)).toBe("€5M");
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
      currency: SUPPORTED_CURRENCIES.GBP,
    });

    expect(formatVal(5000000)).toBe("£4.3M");
    expect(formatVal(500)).toBe("£430");
  });
});

describe("formatExactMoney", () => {
  it("formats exact amounts using the selected settings currency", () => {
    useSettingsStore.setState({
      settings: { ...useSettingsStore.getState().settings, currency: "USD" },
      currency: SUPPORTED_CURRENCIES.USD,
    });

    expect(formatExactMoney(125000)).toBe("$135,000");
    expect(formatExactMoney(-30000)).toBe("-$32,400");
  });
});

describe("formatWeeklyAmount", () => {
  it("appends the weekly unit to a formatted amount", () => {
    expect(formatWeeklyAmount("€10K")).toBe("€10K/wk");
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
