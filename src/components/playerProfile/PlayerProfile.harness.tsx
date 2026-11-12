import { fireEvent, render, screen } from "@testing-library/react";
import { useState } from "react";
import type { ReactElement } from "react";
import type { GameStateData, PlayerData, StaffData, TeamData } from "../../store/gameStore";
import { createSeasonContext } from "../../test-utils/factories";
import PlayerProfile from "./PlayerProfile";

/**
 * TEST-ONLY: imported by the player profile's `*.test.tsx` suites and by nothing in production.
 *
 * The profile's test harness: the fixtures (a team, a player, a staff member, a game), the faked
 * backend's default answers, a parent that really re-renders from the game it is handed back, and
 * the helpers the suites share. The module mocks the suites install are in `PlayerProfile.mocks.ts`.
 */

/**
 * The fields these mocks read out of an invoke payload. Tauri's IPC boundary is untyped by
 * nature — every command takes a different shape — so this names what is actually touched
 * rather than reaching for `any` and switching type-checking off for the whole object.
 */
export type MockInvokePayload = {
  request?: { playerId?: string };
  fee?: number;
  weeklyWage?: number;
};

/** Renders the profile and opens one of its tabs, as a player would to reach what it holds. */
export function renderOnTab(tab: string, ui: ReactElement) {
  const result = render(ui);
  fireEvent.click(screen.getByRole("tab", { name: tab }));
  return result;
}

export function hasWeeklyWage(text: string, amount: string): boolean {
  return text.replace(/\s+/g, "").includes(`€${amount}/wk`);
}

export function createTeam(overrides: Partial<TeamData> = {}): TeamData {
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

export function createPlayer(overrides: Partial<PlayerData> = {}): PlayerData {
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
      appearances: 0,
      goals: 0,
      assists: 0,
      clean_sheets: 0,
      yellow_cards: 0,
      red_cards: 0,
      avg_rating: 0,
      minutes_played: 0,
      shots: 0,
      shots_on_target: 0,
      passes_completed: 0,
      passes_attempted: 0,
      tackles_won: 0,
      interceptions: 0,
      fouls_committed: 0,
    },
    career: [],
    transfer_listed: false,
    loan_listed: false,
    transfer_offers: [],
    movement_history: [],
    traits: [],
    ...overrides,
  };
}

export function createStaff(overrides: Partial<StaffData> = {}): StaffData {
  return {
    id: "staff-1",
    first_name: "Alex",
    last_name: "Assistant",
    display_name: "Alex Assistant",
    date_of_birth: "1980-01-01",
    nationality: "GB",
    role: "AssistantManager" as const,
    attributes: {
      coaching: 70,
      judgingAbility: 60,
      judgingPotential: 60,
      physiotherapy: 20,
    },
    team_id: "team-1",
    specialization: "General",
    wage: 1200,
    contract_end: "2027-06-30",
    ...overrides,
  };
}

export function createGameState(player: PlayerData, staff: StaffData[] = []): GameStateData {
  return {
    season_context: createSeasonContext(),
    clock: {
      current_date: "2026-08-01T00:00:00Z",
      start_date: "2026-07-01T00:00:00Z",
    },
    manager: {
      display_name: "Jane Doe",
      id: "manager-1",
      first_name: "Jane",
      last_name: "Doe",
      date_of_birth: "1980-01-01",
      nationality: "GB",
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
    teams: [createTeam()],
    players: [player],
    staff,
    messages: [],
    news: [],
    league: {
      id: "league-1",
      name: "League",
      season: 1,
      fixtures: [],
      standings: [],
    },
    scouting_assignments: [],
    board_objectives: [],
  };
}

export function createAdvancedStatsSummary() {
  return {
    percentileEligible: false,
    metrics: {
      shots: { total: 0, per90: null, percentile: null },
      shotsOnTarget: { total: 0, per90: null, percentile: null },
      passes: {
        completed: 0,
        attempted: 0,
        accuracy: null,
        percentile: null,
      },
      tacklesWon: { total: 0, per90: null, percentile: null },
      interceptions: { total: 0, per90: null, percentile: null },
      foulsCommitted: { total: 0, per90: null, percentile: null },
    },
  };
}

export function defaultInvokeResponse(command: string) {
  if (command === "get_player_stats_overview") {
    return createAdvancedStatsSummary();
  }

  if (command === "get_player_match_history") {
    return [];
  }

  if (command === "get_scouted_report") {
    return null;
  }

  // The career is the backend's projection; an empty one for a profile that is not about it.
  if (command === "get_player_career") {
    return { by_season: [], by_club: [], totals: { appearances: 0, goals: 0, assists: 0 } };
  }

  return createGameState(createPlayer());
}

export function RenewalHarness({ initialPlayer }: { initialPlayer?: PlayerData }) {
  const [gameState, setGameState] = useState<GameStateData>(
    createGameState(initialPlayer ?? createPlayer()),
  );

  return (
    <PlayerProfile
      player={gameState.players[0]}
      gameState={gameState}
      isOwnClub
      onGameUpdate={setGameState}
    />
  );
}
