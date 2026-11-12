import { fireEvent, screen } from "@testing-library/react";
import { useState } from "react";
import { vi } from "vitest";
import { invoke } from "@tauri-apps/api/core";
import type { CustomTactic } from "../../services/customTacticsService";
import type { GameStateData, PlayerData, TeamData } from "../../store/gameStore";
import { createSeasonContext } from "../../test-utils/factories";
import { familiarityState } from "../../test-utils/tacticsFamiliarityState";
import TacticsTab from "./TacticsTab";

/**
 * TEST-ONLY: imported by the Tactics tab's `*.test.tsx` suites and by nothing in production, so the
 * `invoke` import below is the stand-in the suites mock, as the other component tests on develop mock
 * `@tauri-apps/api/core` directly (the rule that keeps `invoke` in `src/services/` is about
 * production components).
 *
 * The Tactics tab's test harness: a controlled parent that really re-renders from the game a faked
 * backend returns, the faked backend's knobs, and the helpers its suites share. A suite mocks
 * `@tauri-apps/api/core` (so `invoke` here is its stand-in), the familiarity and position hooks, and
 * runs under the real i18n.
 */

export const mockedInvoke = vi.mocked(invoke);

const buildPlayer = (id: string, position: string): PlayerData => ({
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

const makePlayer = (...args: Parameters<typeof buildPlayer>): PlayerData => {
  const player = buildPlayer(...args);
  // The backend would grade him; the stand-in finds him here.
  familiarityState.players.set(player.id, player);
  return player;
};

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
  season_context: createSeasonContext(),
  clock: { current_date: "2026-08-01", start_date: "2026-08-01" },
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

/**
 * What a test turns to steer the faked backend; `resetHarness` puts every one back.
 */
export const knobs = {
  /** The backend's verdict on whether the phase dials still equal the style's blueprint. */
  blueprintModified: false,
  /** Whether the faked backend fails when asked about the blueprint of a preset's style. */
  blueprintStatusFailsForStyle: false,
  /** When set, the faked backend answers about a preset style's blueprint only once this resolves. */
  blueprintStatusGate: null as Promise<void> | null,
  /** How many times the faked backend has answered about a preset style's blueprint. */
  blueprintStatusAnswered: 0,
  /** Commands the faked backend refuses with the career-changed error. */
  refused: new Set<string>(),
  /** What the faked backend says the manager's tactic differs from a saved one in. */
  savedTacticDifferences: {
    formation: false,
    playStyle: false,
    instructions: false,
    eleven: false,
    roles: false,
    takers: false,
  },
  /** When set, the faked backend answers how the tactic differs from a saved one only once this resolves. */
  savedTacticDifferencesGate: null as Promise<void> | null,
  /** Whether the faked backend fails when asked how the tactic differs from a saved one. */
  savedTacticDifferencesFail: false,
  /** How many times the faked backend has answered how the tactic differs from a saved one. */
  savedTacticDifferencesAnswered: 0,
  /** When set, the faked save answers `list_custom_tactics` only once this resolves. */
  libraryListGate: null as Promise<void> | null,
  /** When set, each `list_custom_tactics` waits for its own release, pushed here in the order asked. */
  libraryListReleases: null as Array<() => void> | null,
  /** Whether the next `list_custom_tactics` (after its gate) fails once. */
  libraryListFailsOnce: false,
};

/** The faked backend's own copy of the game, and the tactics its save holds, in the order made. */
let backend: GameStateData;
let savedTactics: CustomTactic[] = [];

/** A copy of the faked backend's game as it stands, for a test that updates the tab from outside. */
export function currentGame(): GameStateData {
  return structuredClone(backend);
}

/** Puts the faked backend back as it starts: a fresh game, an empty save, every knob at rest. */
export function resetHarness() {
  backend = makeGame();
  savedTactics = [];
  knobs.blueprintModified = false;
  knobs.blueprintStatusFailsForStyle = false;
  knobs.blueprintStatusGate = null;
  knobs.blueprintStatusAnswered = 0;
  knobs.refused = new Set();
  knobs.savedTacticDifferences = {
    formation: false,
    playStyle: false,
    instructions: false,
    eleven: false,
    roles: false,
    takers: false,
  };
  knobs.savedTacticDifferencesGate = null;
  knobs.savedTacticDifferencesFail = false;
  knobs.savedTacticDifferencesAnswered = 0;
  knobs.libraryListGate = null;
  knobs.libraryListReleases = null;
  knobs.libraryListFailsOnce = false;
}

/** Puts tactics in the faked save, as if the player had made them earlier. */
export function seedSavedTactics(
  entries: Array<{
    id: string;
    name: string;
    description: string;
    formation: string;
    playStyle: string;
  }>,
) {
  savedTactics = entries.map((entry) => ({ ...entry, sourcePresetName: null }));
}

export function fakeBackend(failPreset = false) {
  mockedInvoke.mockImplementation(async (command: string, args?: unknown) => {
    const params = (args ?? {}) as Record<string, string>;
    if (knobs.refused.has(command)) throw "be.error.careerChanged";
    if (command === "get_squad") return backend.players;
    if (command === "auto_select_set_pieces") {
      return { captain: "m1", penalty_taker: "f1", free_kick_taker: "m1", corner_taker: "d1" };
    }
    if (command === "get_custom_tactic_differences") {
      if (knobs.savedTacticDifferencesGate) await knobs.savedTacticDifferencesGate;
      if (knobs.savedTacticDifferencesFail) throw new Error("the save did not answer");
      knobs.savedTacticDifferencesAnswered += 1;
      return { ...knobs.savedTacticDifferences };
    }
    if (command === "list_custom_tactics") {
      if (knobs.libraryListGate) await knobs.libraryListGate;
      const releases = knobs.libraryListReleases;
      if (releases) await new Promise<void>((release) => releases.push(release));
      if (knobs.libraryListFailsOnce) {
        knobs.libraryListFailsOnce = false;
        throw new Error("the save did not answer");
      }
      return structuredClone(savedTactics);
    }
    if (command === "rename_custom_tactic") {
      const saved = savedTactics.find((tactic) => tactic.id === params.id);
      if (!saved) throw "be.error.customTactic.notFound";
      if (!params.name.trim()) throw "be.error.customTactic.nameRequired";
      saved.name = params.name;
      return structuredClone(savedTactics);
    }
    if (command === "save_custom_tactic" || command === "save_current_tactic") {
      const tactic = (args as { tactic: CustomTactic }).tactic;
      const at = savedTactics.findIndex((saved) => saved.id === tactic.id);
      if (at === -1) savedTactics.push(tactic);
      else savedTactics[at] = tactic;
      return structuredClone(savedTactics);
    }
    const team = backend.teams[0];
    if (command === "get_tactics_blueprint_status") {
      // The verdict is the backend's at the moment it was asked, as the real command's is: a gate
      // holds the reply, not the question.
      const modified = knobs.blueprintModified;
      if (params.playStyle && knobs.blueprintStatusGate) await knobs.blueprintStatusGate;
      if (knobs.blueprintStatusFailsForStyle && params.playStyle) throw new Error("no answer");
      if (params.playStyle) knobs.blueprintStatusAnswered += 1;
      return { modified };
    }
    if (command === "set_tactics_phase") {
      team.tactics_phase = {
        ...(team.tactics_phase as object),
        width: params.width,
      } as TeamData["tactics_phase"];
      knobs.blueprintModified = true;
    }
    if (command === "apply_tactic_preset") {
      if (failPreset) throw new Error("boom");
      team.formation = params.formation;
      team.play_style = params.playStyle;
      // The real command puts the dials back to the style's blueprint, unless the manager kept theirs.
      if (!(params as { keepInstructions?: boolean }).keepInstructions) {
        team.tactics_phase = {} as TeamData["tactics_phase"];
        knobs.blueprintModified = false;
      }
    }
    if (command === "set_formation") team.formation = params.formation;
    if (command === "set_play_style") team.play_style = params.playStyle;
    // As the real command: the saved tactic's formation and style, and nothing else for one saved
    // before tactics held their contents (these seeds hold none), so the dials stay the manager's.
    if (command === "apply_custom_tactic") {
      const saved = savedTactics.find((tactic) => tactic.id === params.id);
      if (!saved) throw new Error("be.error.customTactic.notFound");
      team.formation = saved.formation;
      team.play_style = saved.playStyle;
    }
    return structuredClone(backend);
  });
}

export function ControlledTacticsTab() {
  const [game, setGame] = useState(() => structuredClone(backend));
  return <TacticsTab gameState={game} onSelectPlayer={vi.fn()} onGameUpdate={setGame} />;
}

/** The with-the-ball dials sit in a section of the instructions panel; open it when it is closed. */
export function openPhaseBlueprint() {
  const toggle = screen.getByRole("button", { name: /With Ball/i });
  if (toggle.getAttribute("aria-expanded") !== "true") fireEvent.click(toggle);
}

/** Opens the tactic bar's one actions menu (New, Duplicate, Reset to defaults, Delete). */
export function openActionsMenu() {
  fireEvent.click(screen.getByRole("button", { name: "Tactic actions" }));
}

export const commandsCalled = () => mockedInvoke.mock.calls.map(([command]) => command);
