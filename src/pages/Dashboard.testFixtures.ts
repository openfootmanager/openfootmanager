import type { GameStateData } from "../store/gameStore";
import type { LeagueData, SeasonContextData } from "../store/types";

import { createSeasonContext } from "../test-utils/factories";

export function createGameState(): GameStateData {
  return {
    season_context: createSeasonContext(),
    clock: {
      current_date: "2026-07-10T12:00:00Z",
      start_date: "2026-07-01T12:00:00Z",
    },
    manager: {
      display_name: "Jane Doe",
      id: "manager-1",
      first_name: "Jane",
      last_name: "Doe",
      date_of_birth: "1980-01-01",
      nationality: "England",
      reputation: 50,
      satisfaction: 50,
      fan_approval: 50,
      team_id: "team-1",
      career_stats: {
        matches_managed: 0,
        wins: 0,
        draws: 0,
        losses: 0,
        trophies: 0,
        best_finish: null,
      },
      career_history: [],
    },
    teams: [
      {
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
      },
      {
        id: "team-2",
        name: "Beta FC",
        short_name: "BET",
        country: "GB",
        city: "Manchester",
        stadium_name: "Beta Ground",
        stadium_capacity: 28000,
        finance: 400000,
        manager_id: "manager-2",
        reputation: 48,
        wage_budget: 45000,
        transfer_budget: 200000,
        season_income: 0,
        season_expenses: 0,
        formation: "4-3-3",
        play_style: "Balanced",
        training_focus: "General",
        training_intensity: "Balanced",
        training_schedule: "Balanced",
        founded_year: 1901,
        colors: { primary: "#111111", secondary: "#eeeeee" },
        starting_xi_ids: [],
        form: [],
        history: [],
      },
    ],
    players: [
      {
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
      },
    ],
    staff: [],
    messages: [],
    news: [],
    league: null,
    scouting_assignments: [],
    board_objectives: [],
    extra_translations: {
      en: { tournaments: { customCup: "Custom Cup" } },
    },
  };
}

export function createBackendFinanceSnapshot() {
  return {
    committed_transfer_fees: 0,
    available_transfer_budget: 300000,
    annual_wage_bill: 12000,
    weekly_wage_spend: 12000,
    weekly_wage_budget: 50000,
    weekly_recurring_income: 12000,
    weekly_sponsor_income: 12000,
    projected_weekly_net: 0,
    cash_runway_weeks: null as number | null,
    wage_budget_usage_percent: 24,
    currently_in_debt: false,
    currently_over_budget: false,
    wage_budget_status: "stable",
    runway_status: "stable",
    overall_status: "stable",
    marketing_campaign_cooldown_days_remaining: 0,
    cash_flow: [],
  };
}

/**
 * A Storage that records every key read, installed as `window.localStorage` until `restore`: the
 * object the dashboard reads, whatever Storage the runtime itself provides.
 */
export function installRecordingStorage(): { reads: string[]; restore: () => void } {
  const reads: string[] = [];
  const values = new Map<string, string>();
  const storage: Storage = {
    get length() {
      return values.size;
    },
    clear: () => values.clear(),
    getItem: (key: string) => {
      reads.push(key);
      return values.get(key) ?? null;
    },
    key: (index: number) => [...values.keys()][index] ?? null,
    removeItem: (key: string) => {
      values.delete(key);
    },
    setItem: (key: string, value: string) => {
      values.set(key, String(value));
    },
  };
  const own = Object.getOwnPropertyDescriptor(window, "localStorage");
  Object.defineProperty(window, "localStorage", { configurable: true, value: storage });
  return {
    reads,
    restore: () => {
      if (own) {
        Object.defineProperty(window, "localStorage", own);
      } else {
        Reflect.deleteProperty(window, "localStorage");
      }
    },
  };
}

// A finished league that is not the player's own, sorting first in the array.
// Competitions are ordered by country code, so in a generated world this is
// always Argentina — a split-season country on a different calendar.
export function foreignLeagueStillPlaying(): LeagueData {
  return {
    id: "ar-d1-apertura",
    name: "ar-d1-apertura",
    season: 2035,
    kind: "League",
    scope: "Domestic",
    country_id: "AR",
    priority: 0,
    participant_ids: ["ar-00", "ar-01"],
    rules: { format: "LeagueTable", counts_in_season_flow: true },
    fixtures: [
      {
        id: "ar-1",
        matchday: 1,
        date: "2036-02-10",
        home_team_id: "ar-00",
        away_team_id: "ar-01",
        competition: "League",
        competitive: true,
        advancing_team_id: null,
        status: "Completed",
        result: null,
      },
      {
        id: "ar-2",
        matchday: 2,
        date: "2036-10-10",
        home_team_id: "ar-01",
        away_team_id: "ar-00",
        competition: "League",
        competitive: true,
        advancing_team_id: null,
        status: "Scheduled",
        result: null,
      },
    ],
    standings: [],
  } as LeagueData;
}

export function seasonContext(seasonComplete: boolean): SeasonContextData {
  return {
    phase: seasonComplete ? "PostSeason" : "InSeason",
    season_complete: seasonComplete,
    season_start: null,
    season_end: null,
    days_until_season_start: null,
    transfer_window: {
      status: "Closed",
      opens_on: null,
      closes_on: null,
      days_until_opens: null,
      days_remaining: null,
    },
  } as SeasonContextData;
}
