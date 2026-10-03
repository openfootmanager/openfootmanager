import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { useState } from "react";
import { beforeAll, beforeEach, describe, expect, it, vi } from "vitest";
import { invoke } from "@tauri-apps/api/core";
import { i18nReady } from "../../i18n";
import type { GameStateData, PlayerData, TeamData } from "../../store/gameStore";
import TacticsTab from "./TacticsTab";

/**
 * #365 — picking a tactic preset asks the backend for the whole preset
 * (formation, play style and its phase blueprint) in one command, while the
 * play-style dropdown on its own leaves the manager's dials alone.
 *
 * Real i18n hook, and a controlled parent that really re-renders from the
 * game the (faked) backend returns.
 */

vi.mock("@tauri-apps/api/core", () => ({ invoke: vi.fn() }));
const mockedInvoke = vi.mocked(invoke);

const makePlayer = (id: string, position: string): PlayerData => ({
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
});

const makeTeam = (): TeamData => ({
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
  starting_xi_ids: ["gk1", "d1", "d2", "d3", "d4", "m1", "m2", "m3", "m4", "f1", "f2"],
  form: [],
  history: [],
});

const makeGame = (): GameStateData => ({
  clock: { current_date: "2026-08-01", start_date: "2026-08-01" },
  manager: {
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
  teams: [makeTeam()],
  players: [
    makePlayer("gk1", "Goalkeeper"),
    ...["d1", "d2", "d3", "d4"].map((id) => makePlayer(id, "Defender")),
    ...["m1", "m2", "m3", "m4"].map((id) => makePlayer(id, "Midfielder")),
    ...["f1", "f2"].map((id) => makePlayer(id, "Forward")),
  ],
  staff: [],
  messages: [],
  news: [],
  league: null,
  scouting_assignments: [],
  board_objectives: [],
});

/** The faked backend: applies commands to its own copy and returns it. */
let backend: GameStateData;

function fakeBackend(failPreset = false) {
  mockedInvoke.mockImplementation(async (command: string, args?: unknown) => {
    const params = (args ?? {}) as Record<string, string>;
    if (command === "get_squad") return backend.players;
    const team = backend.teams[0];
    if (command === "apply_tactic_preset") {
      if (failPreset) throw new Error("boom");
      team.formation = params.formation;
      team.play_style = params.playStyle;
    }
    if (command === "set_formation") team.formation = params.formation;
    if (command === "set_play_style") team.play_style = params.playStyle;
    return structuredClone(backend);
  });
}

function ControlledTacticsTab() {
  const [game, setGame] = useState(() => structuredClone(backend));
  return <TacticsTab gameState={game} onSelectPlayer={vi.fn()} onGameUpdate={setGame} />;
}

const commandsCalled = () => mockedInvoke.mock.calls.map(([command]) => command);

describe("TacticsTab preset selection", () => {
  beforeAll(async () => {
    await i18nReady;
  });

  beforeEach(() => {
    localStorage.clear();
    mockedInvoke.mockReset();
    backend = makeGame();
  });

  /**
   * Given a manager on 4-4-2 Balanced
   * When they pick the High Press preset
   * Then the whole preset is requested in one apply_tactic_preset call, with
   *      neither set_formation nor set_play_style, and the tactic shows as active
   */
  it("applies a preset with one apply_tactic_preset command", async () => {
    fakeBackend();
    render(<ControlledTacticsTab />);

    fireEvent.click(screen.getByRole("button", { name: "Choose tactic" }));
    fireEvent.click(screen.getByRole("option", { name: /High Press/ }));

    await waitFor(() => {
      expect(mockedInvoke).toHaveBeenCalledWith("apply_tactic_preset", {
        formation: "3-4-3",
        playStyle: "HighPress",
      });
    });
    expect(commandsCalled()).not.toContain("set_formation");
    expect(commandsCalled()).not.toContain("set_play_style");
    await waitFor(() => {
      expect(screen.getByRole("button", { name: "Choose tactic" })).toHaveTextContent("High Press");
    });
  });

  /**
   * Given a manager on the Balanced Control preset
   * When they change only the play style dropdown
   * Then set_play_style is used and the preset command is not
   */
  it("changing the play style alone does not ask for a preset", async () => {
    fakeBackend();
    render(<ControlledTacticsTab />);

    fireEvent.click(screen.getByRole("combobox", { name: "Play Style" }));
    fireEvent.click(screen.getByRole("option", { name: /Defensive/ }));

    await waitFor(() => {
      expect(mockedInvoke).toHaveBeenCalledWith("set_play_style", { playStyle: "Defensive" });
    });
    expect(commandsCalled()).not.toContain("apply_tactic_preset");
  });

  /**
   * Given the backend refuses the preset
   * When the manager picks it
   * Then the previously active tactic stays active
   */
  it("keeps the previous tactic when the preset command fails", async () => {
    fakeBackend(true);
    vi.spyOn(console, "error").mockImplementation(() => undefined);
    render(<ControlledTacticsTab />);

    fireEvent.click(screen.getByRole("button", { name: "Choose tactic" }));
    fireEvent.click(screen.getByRole("option", { name: /High Press/ }));

    await waitFor(() => {
      expect(mockedInvoke).toHaveBeenCalledWith("apply_tactic_preset", expect.anything());
    });
    expect(screen.getByRole("button", { name: "Choose tactic" })).toHaveTextContent(
      "Balanced Control",
    );
  });
});
