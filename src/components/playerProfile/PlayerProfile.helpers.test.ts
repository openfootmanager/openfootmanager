import { afterEach, beforeEach, describe, expect, it } from "vitest";

import type { PlayerData, TeamData } from "../../store/gameStore";
import { useSettingsStore } from "../../store/settingsStore";
import {
  buildPlayerAdvancedStats,
  formatPlayerWageCompact,
  formatPlayerMarketValue,
  formatPlayerWage,
  getAttributeColorClass,
  getPlayerTeamName,
  resolvePlayerInjuryName,
} from "./PlayerProfile.helpers";

function createTeam(overrides: Partial<TeamData> = {}): TeamData {
  return {
    id: "team-1",
    name: "Alpha FC",
    short_name: "ALP",
    country: "GB",
    city: "London",
    stadium_name: "Alpha Ground",
    stadium_capacity: 30000,
    finance: 500000,
    manager_id: "manager-1",
    reputation: 50,
    wage_budget: 50000,
    transfer_budget: 250000,
    season_income: 0,
    season_expenses: 0,
    formation: "4-4-2",
    play_style: "Balanced",
    training_focus: "General",
    training_intensity: "Balanced",
    training_schedule: "Balanced",
    founded_year: 1900,
    colors: { primary: "#000000", secondary: "#ffffff" },
    starting_xi_ids: [],
    form: [],
    history: [],
    ...overrides,
  };
}

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

function createPlayer(overrides: Partial<PlayerData> = {}): PlayerData {
  return {
    id: "player-1",
    match_name: "J. Smith",
    full_name: "John Smith",
    date_of_birth: "2000-01-01",
    nationality: "GB",
    position: "Forward",
    natural_position: "Forward",
    alternate_positions: [],
    training_focus: null,
    attributes: {
      pace: 60,
      stamina: 60,
      strength: 60,
      agility: 60,
      passing: 60,
      shooting: 60,
      tackling: 60,
      dribbling: 60,
      defending: 60,
      positioning: 60,
      vision: 60,
      decisions: 60,
      composure: 60,
      aggression: 60,
      teamwork: 60,
      leadership: 60,
      handling: 20,
      reflexes: 20,
      aerial: 60,
    },
    condition: 80,
    morale: 75,
    injury: null,
    team_id: "team-1",
    retired: false,
    contract_end: "2026-10-15",
    wage: 12000,
    market_value: 350000,
    stats: {
      appearances: 10,
      goals: 4,
      assists: 3,
      clean_sheets: 0,
      yellow_cards: 1,
      red_cards: 0,
      avg_rating: 7.2,
      minutes_played: 450,
      shots: 20,
      shots_on_target: 10,
      passes_completed: 80,
      passes_attempted: 100,
      tackles_won: 9,
      interceptions: 6,
      fouls_committed: 5,
    },
    career: [],
    transfer_listed: false,
    loan_listed: false,
    transfer_offers: [],
    traits: [],
    ...overrides,
  };
}

describe("PlayerProfile.helpers", (): void => {
  it("resolves the player team name with free-agent and unknown fallbacks", (): void => {
    const teams = [createTeam()];

    expect(
      getPlayerTeamName(teams, "team-1", {
        freeAgent: "Free Agent",
        unknown: "Unknown",
      }),
    ).toBe("Alpha FC");
    expect(
      getPlayerTeamName(teams, null, {
        freeAgent: "Free Agent",
        unknown: "Unknown",
      }),
    ).toBe("Free Agent");
    expect(
      getPlayerTeamName(teams, "team-2", {
        freeAgent: "Free Agent",
        unknown: "Unknown",
      }),
    ).toBe("Unknown");
  });

  it("formats market values across value ranges", (): void => {
    expect(formatPlayerMarketValue(999)).toBe("€999");
    expect(formatPlayerMarketValue(125000)).toBe("€125K");
    expect(formatPlayerMarketValue(2500000)).toBe("€2.5M");
  });

  it("formats stored wages as weekly display values", (): void => {
    expect(formatPlayerWage(1000)).toMatch(/^€1[.,]000\/wk$/);
  });

  it("formats current player wages as weekly display values", (): void => {
    expect(formatPlayerWageCompact(52000)).toBe("€52K/wk");
  });

  it("respects the selected settings currency for market values and wages", (): void => {
    useSettingsStore.setState({
      settings: { ...useSettingsStore.getState().settings, currency: "GBP" },
      currency: SUPPORTED_CURRENCIES.GBP,
    });

    expect(formatPlayerMarketValue(125000)).toBe("£108K");
    expect(formatPlayerWage(1000)).toBe("£860/wk");
  });

  it("maps attribute values to readable shared color classes", (): void => {
    expect(getAttributeColorClass(85)).toContain("text-success-600");
    expect(getAttributeColorClass(70)).toContain("text-success-600");
    expect(getAttributeColorClass(65)).toContain("text-accent-700");
    expect(getAttributeColorClass(45)).toContain("text-accent-700");
    expect(getAttributeColorClass(39)).toContain("text-red-600");
    expect(getAttributeColorClass(20)).toContain("text-red-600");
  });

  it("resolves injury names for explicit keys and plain injuries", (): void => {
    const translate = (key: string, options?: { defaultValue?: unknown }): string => {
      return typeof options?.defaultValue === "string" ? `${key}:${options.defaultValue}` : key;
    };

    expect(resolvePlayerInjuryName("injuries.hamstring", translate)).toBe(
      "injuries.hamstring:injuries.hamstring",
    );
    expect(resolvePlayerInjuryName("Hamstring", translate)).toBe(
      "common.injuries.Hamstring:Hamstring",
    );
  });

  /**
   * Given a player's own season stats and nothing from the backend yet
   * When the placeholder advanced stats are built
   * Then totals and per-90 rates are there, and no percentile or peer group is invented
   */
  it("builds advanced stats from the squad data without percentiles", (): void => {
    const player = createPlayer({
      stats: {
        ...createPlayer().stats,
        minutes_played: 450,
        shots: 20,
        shots_on_target: 10,
        passes_completed: 80,
        passes_attempted: 100,
        tackles_won: 9,
        interceptions: 6,
        fouls_committed: 5,
      },
    });

    const summary = buildPlayerAdvancedStats(player);

    expect(summary.metrics.shots.total).toBe(20);
    expect(summary.metrics.shots.per90).toBe(4);
    expect(summary.metrics.shotsOnTarget.per90).toBe(2);
    expect(summary.metrics.passes.accuracy).toBe(80);
    expect(summary.metrics.tacklesWon.per90).toBe(1.8);
    expect(summary.metrics.interceptions.per90).toBe(1.2);
    expect(summary.metrics.foulsCommitted.per90).toBe(1);
    expect(summary.percentileEligible).toBe(false);
    expect(summary.peerGroup).toBeNull();
    for (const metric of [
      summary.metrics.shots,
      summary.metrics.shotsOnTarget,
      summary.metrics.passes,
      summary.metrics.tacklesWon,
      summary.metrics.interceptions,
      summary.metrics.foulsCommitted,
    ]) {
      expect(metric.percentile).toBeNull();
    }
  });
});
