import { fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import { useState } from "react";
import { beforeAll, beforeEach, describe, expect, it, vi } from "vitest";
import { invoke } from "@tauri-apps/api/core";
import { i18nReady } from "../../i18n";
import type { GameStateData, PlayerData, TeamData } from "../../store/gameStore";
import TacticsTab from "./TacticsTab";
import type { CustomTactic } from "../../services/customTacticsService";
import { createSeasonContext } from "../../test-utils/factories";

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

/** The faked backend: applies commands to its own copy and returns it. */
let backend: GameStateData;
/** The backend's verdict on whether the phase dials still equal the style's blueprint. */
let blueprintModified = false;
/** The tactics the faked save holds, in the order made. */
let savedTactics: CustomTactic[] = [];

/** Puts tactics in the faked save, as if the player had made them earlier. */
function seedSavedTactics(
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

function fakeBackend(failPreset = false) {
  mockedInvoke.mockImplementation(async (command: string, args?: unknown) => {
    const params = (args ?? {}) as Record<string, string>;
    if (command === "get_squad") return backend.players;
    if (command === "list_custom_tactics") return structuredClone(savedTactics);
    if (command === "save_custom_tactic") {
      const tactic = (args as { tactic: CustomTactic }).tactic;
      const at = savedTactics.findIndex((saved) => saved.id === tactic.id);
      if (at === -1) savedTactics.push(tactic);
      else savedTactics[at] = tactic;
      return structuredClone(savedTactics);
    }
    const team = backend.teams[0];
    if (command === "get_tactics_blueprint_status") return { modified: blueprintModified };
    if (command === "set_tactics_phase") {
      team.tactics_phase = {
        ...(team.tactics_phase as object),
        width: params.width,
      } as TeamData["tactics_phase"];
      blueprintModified = true;
    }
    if (command === "apply_tactic_preset") {
      if (failPreset) throw new Error("boom");
      team.formation = params.formation;
      team.play_style = params.playStyle;
      // The real command puts the dials back to the style's blueprint.
      team.tactics_phase = {} as TeamData["tactics_phase"];
      blueprintModified = false;
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

/** The phase dials sit in a collapsible section; open it when it is closed. */
function openPhaseBlueprint() {
  const toggle = screen.getByRole("button", { name: /Phase Blueprint/i });
  if (toggle.getAttribute("aria-expanded") !== "true") fireEvent.click(toggle);
}

const commandsCalled = () => mockedInvoke.mock.calls.map(([command]) => command);

// Every case mounts the whole TacticsTab with the real i18n and then drives several controls; that
// takes 2-5 s on a CI runner under load, and vitest's default 5 s cut-off turned the slowest cases
// into flaky failures. A generous per-describe limit keeps them honest without making them faster
// than the screen they exercise.
const TACTICS_TAB_CASE_TIMEOUT_MS = 20_000;

describe("TacticsTab preset selection", { timeout: TACTICS_TAB_CASE_TIMEOUT_MS }, () => {
  beforeAll(async () => {
    await i18nReady;
  });

  beforeEach(() => {
    localStorage.clear();
    mockedInvoke.mockReset();
    blueprintModified = false;
    savedTactics = [];
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

  /** Given the active Balanced preset, when it is picked again from the list,
   * then the preset command is still requested to restore its blueprint. */
  it("requests a blueprint reset when the active preset is picked again", async () => {
    fakeBackend();
    render(<ControlledTacticsTab />);
    fireEvent.click(screen.getByRole("button", { name: "Choose tactic" }));
    fireEvent.click(screen.getByRole("option", { name: /Balanced Control/ }));
    await waitFor(() =>
      expect(mockedInvoke).toHaveBeenCalledWith("apply_tactic_preset", {
        formation: "4-4-2",
        playStyle: "Balanced",
      }),
    );
  });

  /** Given a saved custom tactic (including one with a preset-like ID), when it
   * is picked and manually adjusted to a built-in setup, then its type keeps
   * the preset reset off and preserves the active custom tactic. */
  it.each(["custom:saved", "preset:imported-custom"])(
    "keeps the blueprint when selecting saved custom tactic %s",
    async (id) => {
      seedSavedTactics([
        {
          id,
          name: "Saved custom",
          description: "Custom setup",
          formation: "3-4-3",
          playStyle: "HighPress",
        },
      ]);
      fakeBackend();
      render(<ControlledTacticsTab />);
      fireEvent.click(screen.getByRole("button", { name: "Choose tactic" }));
      fireEvent.click(await screen.findByRole("option", { name: /Saved custom/ }));
      await waitFor(() =>
        expect(mockedInvoke).toHaveBeenCalledWith("set_play_style", { playStyle: "HighPress" }),
      );
      expect(mockedInvoke).toHaveBeenCalledWith("set_formation", { formation: "3-4-3" });
      expect(commandsCalled()).not.toContain("apply_tactic_preset");
      await waitFor(() =>
        expect(screen.getByRole("button", { name: "Choose tactic" })).toHaveTextContent(
          "Saved custom",
        ),
      );
      fireEvent.click(screen.getByRole("combobox", { name: "Play Style" }));
      fireEvent.click(screen.getByRole("option", { name: /Balanced/ }));
      fireEvent.click(screen.getByRole("combobox", { name: "Formation" }));
      fireEvent.click(screen.getByRole("option", { name: "4-4-2" }));
      await waitFor(() =>
        expect(screen.getByRole("button", { name: "Choose tactic" })).toHaveTextContent(
          "Saved custom",
        ),
      );
      expect(commandsCalled()).not.toContain("apply_tactic_preset");
    },
  );

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

  /**
   * Given a manager on the Balanced Control preset who has moved to a formation no preset uses
   * When they press Reset
   * Then the preset is applied again in one apply_tactic_preset call (its formation, style and
   *      blueprint), and the tactic is still the preset.
   */
  it("resets a customised preset with apply_tactic_preset", async () => {
    fakeBackend();
    render(<ControlledTacticsTab />);
    fireEvent.click(screen.getByRole("combobox", { name: "Formation" }));
    fireEvent.click(screen.getByRole("option", { name: "5-3-2" }));
    await waitFor(() =>
      expect(mockedInvoke).toHaveBeenCalledWith("set_formation", { formation: "5-3-2" }),
    );
    mockedInvoke.mockClear();

    fireEvent.click(screen.getByRole("button", { name: "Reset to defaults" }));

    await waitFor(() =>
      expect(mockedInvoke).toHaveBeenCalledWith("apply_tactic_preset", {
        formation: "4-4-2",
        playStyle: "Balanced",
      }),
    );
    expect(screen.getByRole("button", { name: "Choose tactic" })).toHaveTextContent(
      "Balanced Control",
    );
  });

  /**
   * Given a manager on the Balanced Control preset with nothing changed
   * When they look at Reset
   * Then it is disabled, as the badge says Synced.
   */
  it("disables Reset on an untouched preset", () => {
    fakeBackend();
    render(<ControlledTacticsTab />);

    expect(screen.getByText("Synced")).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Reset to defaults" })).toBeDisabled();
  });

  /**
   * Given a preset whose phase dial was tuned
   * When the manager presses Reset
   * Then Reset was enabled by the tuned dial, apply_tactic_preset restores the blueprint, and both
   *      the badge and the button return to their idle state.
   */
  it("enables Reset when a dial is tuned and clears badge and button after resetting", async () => {
    fakeBackend();
    render(<ControlledTacticsTab />);
    openPhaseBlueprint();
    const widthRow = screen.getByText("Width").parentElement as HTMLElement;
    fireEvent.click(within(widthRow).getByRole("combobox"));
    fireEvent.click(screen.getByRole("option", { name: "Wide" }));
    await waitFor(() => expect(screen.getByText("Unsaved changes")).toBeInTheDocument());
    const reset = screen.getByRole("button", { name: "Reset to defaults" });
    expect(reset).toBeEnabled();

    fireEvent.click(reset);

    await waitFor(() =>
      expect(mockedInvoke).toHaveBeenCalledWith("apply_tactic_preset", {
        formation: "4-4-2",
        playStyle: "Balanced",
      }),
    );
    await waitFor(() => expect(screen.getByText("Synced")).toBeInTheDocument());
    expect(screen.getByRole("button", { name: "Reset to defaults" })).toBeDisabled();
  });

  /**
   * Given a saved custom tactic whose formation was then changed
   * When the manager presses Reset
   * Then the formation goes back to the saved one, and no preset is applied.
   */
  it("resets a customised custom tactic to its saved setup without applying a preset", async () => {
    seedSavedTactics([
      {
        id: "custom:saved",
        name: "Saved custom",
        description: "Custom setup",
        formation: "4-4-2",
        playStyle: "Balanced",
      },
    ]);
    fakeBackend();
    render(<ControlledTacticsTab />);
    fireEvent.click(screen.getByRole("button", { name: "Choose tactic" }));
    fireEvent.click(await screen.findByRole("option", { name: /Saved custom/ }));
    await waitFor(() =>
      expect(screen.getByRole("button", { name: "Choose tactic" })).toHaveTextContent(
        "Saved custom",
      ),
    );
    fireEvent.click(screen.getByRole("combobox", { name: "Formation" }));
    fireEvent.click(screen.getByRole("option", { name: "5-3-2" }));
    await waitFor(() =>
      expect(mockedInvoke).toHaveBeenCalledWith("set_formation", { formation: "5-3-2" }),
    );
    mockedInvoke.mockClear();

    fireEvent.click(screen.getByRole("button", { name: "Reset to defaults" }));

    await waitFor(() =>
      expect(mockedInvoke).toHaveBeenCalledWith("set_formation", { formation: "4-4-2" }),
    );
    expect(commandsCalled()).not.toContain("apply_tactic_preset");
  });

  /**
   * Given a manager on a preset whose formation and style are untouched
   * When they tune a phase dial
   * Then the badge turns from Synced to Unsaved changes, because the backend says the dials no
   *      longer match the style's blueprint.
   */
  it("shows the unsaved-changes badge once a phase dial is tuned", async () => {
    fakeBackend();
    render(<ControlledTacticsTab />);
    expect(screen.getByText("Synced")).toBeInTheDocument();

    openPhaseBlueprint();
    const widthRow = screen.getByText("Width").parentElement as HTMLElement;
    fireEvent.click(within(widthRow).getByRole("combobox"));
    fireEvent.click(screen.getByRole("option", { name: "Wide" }));

    await waitFor(() => expect(screen.getByText("Unsaved changes")).toBeInTheDocument());
  });

  /**
   * Given a preset whose dials were tuned
   * When the manager picks that preset again
   * Then the badge returns to Synced.
   */
  it("clears the badge when the preset is picked again", async () => {
    fakeBackend();
    render(<ControlledTacticsTab />);
    openPhaseBlueprint();
    const widthRow = screen.getByText("Width").parentElement as HTMLElement;
    fireEvent.click(within(widthRow).getByRole("combobox"));
    fireEvent.click(screen.getByRole("option", { name: "Wide" }));
    await waitFor(() => expect(screen.getByText("Unsaved changes")).toBeInTheDocument());

    fireEvent.click(screen.getByRole("button", { name: "Choose tactic" }));
    fireEvent.click(screen.getByRole("option", { name: /Balanced Control/ }));

    await waitFor(() => expect(screen.getByText("Synced")).toBeInTheDocument());
  });
});
