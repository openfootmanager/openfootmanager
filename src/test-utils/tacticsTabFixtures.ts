import { fireEvent, screen } from "@testing-library/react";

import type { GameStateData, PlayerData, TeamData } from "../store/gameStore";
import { createSeasonContext } from "./factories";

/**
 * The Tactics screen's two clubs, as the screen's own tests draw them: a first team with its
 * starting eleven and bench, and the other club's squad. Shared by the Tactics tab's test files.
 */

export const buildPlayer = (
  id: string,
  position: string,
  overrides: Partial<PlayerData> = {},
): PlayerData => ({
  id,
  match_name: id.toUpperCase(),
  full_name: `Player ${id}`,
  date_of_birth: "1998-01-01",
  nationality: "GB",
  position,
  natural_position: position,
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
    handling: 60,
    reflexes: 60,
    aerial: 60,
  },
  condition: 100,
  morale: 80,
  injury: null,
  // The backend's verdict for the match day (a healthy senior is eligible); a test that needs a
  // different verdict says so.
  match_day_eligible: true,
  team_id: "team1",
  retired: false,
  contract_end: "2027-06-30",
  wage: 1000,
  market_value: 100000,
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

/**
 * The players the backend's familiarity grades, and whether it has arrived. The test files' stand-in
 * for the familiarity hook reads this (every player built by `makePlayer` is registered here), and a
 * test sets `arrived` to say it has not.
 */
export const familiarityState = { arrived: true, players: new Map<string, PlayerData>() };

export const makePlayer = (...args: Parameters<typeof buildPlayer>): PlayerData => {
  const player = buildPlayer(...args);
  // The backend would grade him; the stand-in finds him here.
  familiarityState.players.set(player.id, player);
  return player;
};

export const makeTeam = (overrides: Partial<TeamData> = {}): TeamData => ({
  id: "team1",
  name: "Test FC",
  short_name: "TFC",
  country: "England",
  city: "Test City",
  stadium_name: "Test Ground",
  stadium_capacity: 20000,
  finance: 1000000,
  manager_id: "mgr1",
  reputation: 50,
  wage_budget: 100000,
  transfer_budget: 500000,
  season_income: 0,
  season_expenses: 0,
  formation: "4-4-2",
  play_style: "Balanced",
  training_focus: "General",
  training_intensity: "Balanced",
  training_schedule: "Balanced",
  founded_year: 1900,
  colors: { primary: "#00ff00", secondary: "#ffffff" },
  starting_xi_ids: [],
  form: [],
  history: [],
  ...overrides,
});

export const makeGameState = (): GameStateData => {
  const players = [
    makePlayer("gk1", "Goalkeeper"),
    makePlayer("d1", "Center Back", {
      attributes: {
        pace: 50,
        stamina: 60,
        strength: 70,
        agility: 55,
        passing: 52,
        shooting: 35,
        tackling: 75,
        dribbling: 45,
        defending: 78,
        positioning: 68,
        vision: 50,
        decisions: 63,
        composure: 64,
        aggression: 71,
        teamwork: 62,
        leadership: 60,
        handling: 10,
        reflexes: 10,
        aerial: 15,
      },
    }),
    makePlayer("d2", "Defender"),
    makePlayer("d3", "Defender"),
    makePlayer("d4", "Defender"),
    makePlayer("m1", "Midfielder", {
      attributes: {
        pace: 70,
        stamina: 74,
        strength: 58,
        agility: 75,
        passing: 79,
        shooting: 66,
        tackling: 61,
        dribbling: 77,
        defending: 57,
        positioning: 72,
        vision: 80,
        decisions: 78,
        composure: 73,
        aggression: 52,
        teamwork: 81,
        leadership: 64,
        handling: 10,
        reflexes: 10,
        aerial: 10,
      },
    }),
    makePlayer("m2", "Midfielder"),
    makePlayer("m3", "Midfielder"),
    makePlayer("m4", "Midfielder"),
    makePlayer("f1", "Forward"),
    makePlayer("f2", "Forward"),
    makePlayer("d5", "Defender", { match_name: "Bench DEF" }),
  ];

  return {
    season_context: createSeasonContext(),
    clock: {
      current_date: "2026-08-01",
      start_date: "2026-08-01",
    },
    manager: {
      display_name: "Test Manager",
      id: "mgr1",
      first_name: "Test",
      last_name: "Manager",
      date_of_birth: "1980-01-01",
      nationality: "GB",
      reputation: 50,
      satisfaction: 50,
      fan_approval: 50,
      team_id: "team1",
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
      makeTeam({
        starting_xi_ids: ["gk1", "d1", "d2", "d3", "d4", "m1", "m2", "m3", "m4", "f1", "f2"],
      }),
    ],
    players,
    staff: [],
    messages: [],
    news: [],
    league: null,
    scouting_assignments: [],
    board_objectives: [],
  };
};

/**
 * Swaps the bench defender d5 in for the centre back d1 the way the screen offers it: pick one in
 * the table, pick the other, then confirm in the comparison.
 */
export const swapBenchDefenderIn = () => {
  fireEvent.click(screen.getByTestId("pitch-bench-player-d5"));
  fireEvent.click(screen.getByTestId("xi-player-d1"));
  fireEvent.click(screen.getByRole("button", { name: "tactics.confirmSwap" }));
};

/** Every element under `root` a keyboard could reach: native controls and anything given a tab stop. */
export function focusableIn(root: HTMLElement): HTMLElement[] {
  return Array.from(
    root.querySelectorAll<HTMLElement>(
      'a[href], button, input, select, textarea, summary, [contenteditable="true"], [tabindex]:not([tabindex="-1"])',
    ),
  );
}
