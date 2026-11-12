import { fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import { StrictMode, useState } from "react";
import { beforeAll, beforeEach, describe, expect, it, vi } from "vitest";
import { invoke } from "@tauri-apps/api/core";
import { i18nReady } from "../../i18n";
import type { GameStateData, PlayerData, TeamData } from "../../store/gameStore";
import TacticsTab from "./TacticsTab";
import { resetTacticsLineupCache } from "../../hooks/useTacticsLineupProjection";
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

// The players the backend's familiarity grades, and whether it has arrived (every player built here).
const familiarityState = vi.hoisted(() => ({
  arrived: true,
  players: new Map<string, unknown>(),
}));

vi.mock("../../hooks/useFamiliarity", async () => {
  const { cannedFamiliarityModule } = await import("../../test-utils/familiarity");
  return cannedFamiliarityModule(
    (playerId) => familiarityState.players.get(playerId) as PlayerData | undefined,
    () => familiarityState.arrived,
  );
});

vi.mock("../../hooks/usePositionGroups", async () => {
  const { cannedPositionsModule } = await import("../../test-utils/positionTable");
  return cannedPositionsModule();
});

vi.mock("@tauri-apps/api/core", () => ({ invoke: vi.fn() }));
const mockedInvoke = vi.mocked(invoke);

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

/** The faked backend: applies commands to its own copy and returns it. */
let backend: GameStateData;
/** The backend's verdict on whether the phase dials still equal the style's blueprint. */
let blueprintModified = false;
/** Commands the faked backend refuses with the career-changed error. */
let refused = new Set<string>();
/** The tactics the faked save holds, in the order made. */
let savedTactics: CustomTactic[] = [];
/** When set, the faked save answers `list_custom_tactics` only once this resolves. */
let libraryListGate: Promise<void> | null = null;
/** When set, each `list_custom_tactics` waits for its own release, pushed here in the order asked. */
let libraryListReleases: Array<() => void> | null = null;
/** Whether the next `list_custom_tactics` (after its gate) fails once. */
let libraryListFailsOnce = false;

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
    if (refused.has(command)) throw "be.error.careerChanged";
    if (command === "get_squad") return backend.players;
    if (command === "auto_select_set_pieces") {
      return { captain: "m1", penalty_taker: "f1", free_kick_taker: "m1", corner_taker: "d1" };
    }
    if (command === "list_custom_tactics") {
      if (libraryListGate) await libraryListGate;
      const releases = libraryListReleases;
      if (releases) await new Promise<void>((release) => releases.push(release));
      if (libraryListFailsOnce) {
        libraryListFailsOnce = false;
        throw new Error("the save did not answer");
      }
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

function ControlledTacticsTab() {
  const [game, setGame] = useState(() => structuredClone(backend));
  return <TacticsTab gameState={game} onSelectPlayer={vi.fn()} onGameUpdate={setGame} />;
}

/** The with-the-ball dials sit in a section of the instructions panel; open it when it is closed. */
function openPhaseBlueprint() {
  const toggle = screen.getByRole("button", { name: /With Ball/i });
  if (toggle.getAttribute("aria-expanded") !== "true") fireEvent.click(toggle);
}

/** Opens the tactic bar's one actions menu (New, Duplicate, Reset to defaults, Delete). */
function openActionsMenu() {
  fireEvent.click(screen.getByRole("button", { name: "Tactic actions" }));
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
    resetTacticsLineupCache();
    mockedInvoke.mockReset();
    blueprintModified = false;
    savedTactics = [];
    libraryListGate = null;
    libraryListReleases = null;
    libraryListFailsOnce = false;
    refused = new Set();
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
        careerToken: 0,
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
   * Given the save has not yet answered the library's first load
   * When the manager picks a preset (whose Save is always on once the library is there)
   * Then Save stays disabled, and turns on once the load has answered.
   */
  it("keeps Save disabled until the library's first load has answered", async () => {
    let answerLoad!: () => void;
    libraryListGate = new Promise<void>((resolve) => {
      answerLoad = resolve;
    });
    fakeBackend();
    render(<ControlledTacticsTab />);

    fireEvent.click(screen.getByRole("button", { name: "Choose tactic" }));
    fireEvent.click(screen.getByRole("option", { name: /High Press/ }));
    await waitFor(() => {
      expect(screen.getByRole("button", { name: "Choose tactic" })).toHaveTextContent("High Press");
    });
    expect(screen.getByRole("button", { name: "Save as tactic" })).toBeDisabled();

    answerLoad();

    await waitFor(() =>
      expect(screen.getByRole("button", { name: "Save as tactic" })).toBeEnabled(),
    );
  });

  /**
   * Given the screen under StrictMode, which asks for the library twice (two loads queued in order)
   * When the first load answers while the second is still pending
   * Then Save stays disabled, and turns on only when the second has answered too.
   */
  it("keeps Save disabled until the latest of two overlapping library loads has answered", async () => {
    libraryListReleases = [];
    const releases = libraryListReleases;
    fakeBackend();
    render(
      <StrictMode>
        <ControlledTacticsTab />
      </StrictMode>,
    );
    fireEvent.click(screen.getByRole("button", { name: "Choose tactic" }));
    fireEvent.click(screen.getByRole("option", { name: /High Press/ }));
    await waitFor(() => expect(releases).toHaveLength(1));

    releases[0]();
    await waitFor(() => expect(releases).toHaveLength(2));

    expect(screen.getByRole("button", { name: "Save as tactic" })).toBeDisabled();
    releases[1]();
    await waitFor(() =>
      expect(screen.getByRole("button", { name: "Save as tactic" })).toBeEnabled(),
    );
  });

  /**
   * Given a first load of the library that fails
   * When it fails
   * Then Save turns on (it is how a tactic gets into the save), and a save is accepted.
   */
  it("turns Save on when the library's first load fails, and a save then lands", async () => {
    let answerLoad!: () => void;
    libraryListGate = new Promise<void>((resolve) => {
      answerLoad = resolve;
    });
    libraryListFailsOnce = true;
    fakeBackend();
    render(<ControlledTacticsTab />);
    fireEvent.click(screen.getByRole("button", { name: "Choose tactic" }));
    fireEvent.click(screen.getByRole("option", { name: /High Press/ }));
    await waitFor(() => {
      expect(screen.getByRole("button", { name: "Choose tactic" })).toHaveTextContent("High Press");
    });
    expect(screen.getByRole("button", { name: "Save as tactic" })).toBeDisabled();

    answerLoad();

    await waitFor(() =>
      expect(screen.getByRole("button", { name: "Save as tactic" })).toBeEnabled(),
    );
    fireEvent.click(screen.getByRole("button", { name: "Save as tactic" }));
    await waitFor(() => expect(commandsCalled()).toContain("save_current_tactic"));
    expect(screen.queryByRole("alert")).toBeNull();
  });

  /**
   * Given a backend that refuses a tactic write (the career changed under the screen)
   * When the manager picks a preset, a saved tactic, a formation, a play style, a phase dial,
   *      swaps two starters, sets a player role or auto-selects the set-piece takers
   * Then a translated alert says it did not save (and the write was really asked for, so the
   *      refusal was reached).
   */
  it.each([
    [
      "a preset",
      "apply_tactic_preset",
      async () => {
        fireEvent.click(screen.getByRole("button", { name: "Choose tactic" }));
        fireEvent.click(screen.getByRole("option", { name: /High Press/ }));
      },
    ],
    [
      "a saved tactic",
      "apply_custom_tactic",
      async () => {
        fireEvent.click(screen.getByRole("button", { name: "Choose tactic" }));
        fireEvent.click(await screen.findByRole("option", { name: /Saved custom/ }));
      },
    ],
    [
      "a formation",
      "set_formation",
      async () => {
        fireEvent.click(screen.getByRole("combobox", { name: "Formation" }));
        fireEvent.click(screen.getByRole("option", { name: "3-4-3" }));
      },
    ],
    [
      "a play style",
      "set_play_style",
      async () => {
        fireEvent.click(screen.getByRole("combobox", { name: "Play style" }));
        fireEvent.click(screen.getByRole("option", { name: "Counter" }));
      },
    ],
    [
      "a phase dial",
      "set_tactics_phase",
      async () => {
        openPhaseBlueprint();
        const widthRow = screen.getByText("Width").parentElement as HTMLElement;
        fireEvent.click(within(widthRow).getByRole("combobox"));
        fireEvent.click(screen.getByRole("option", { name: "Wide" }));
      },
    ],
    [
      "a swap of two starters",
      "set_starting_xi",
      async () => {
        fireEvent.click(await screen.findByTestId("pitch-player-d1"));
        fireEvent.click(screen.getByTestId("pitch-player-d2"));
        fireEvent.click(await screen.findByRole("button", { name: "Confirm swap" }));
      },
    ],
    [
      "a player role",
      "set_player_role",
      async () => {
        const token = await screen.findByTestId("pitch-player-d1");
        fireEvent.click(within(token).getByRole("combobox"));
        fireEvent.click(screen.getAllByRole("option")[1]);
      },
    ],
    [
      "a player role chosen in the table",
      "set_player_role",
      async () => {
        const select = (await screen.findAllByRole("combobox")).find((control) =>
          control.getAttribute("aria-label")?.startsWith("Role:"),
        ) as HTMLElement;
        fireEvent.click(select);
        fireEvent.click(screen.getAllByRole("option")[1]);
      },
    ],
    [
      "the auto-selected set-piece takers",
      "set_team_match_roles",
      async () => {
        fireEvent.click(await screen.findByRole("button", { name: "Auto-select defaults" }));
      },
    ],
  ])("tells the manager that %s did not save", async (_name, command, pick) => {
    seedSavedTactics([
      {
        id: "custom:saved",
        name: "Saved custom",
        description: "Custom setup",
        formation: "3-4-3",
        playStyle: "HighPress",
      },
    ]);
    refused = new Set([command]);
    fakeBackend();
    render(<ControlledTacticsTab />);

    await pick();

    const alert = await screen.findByRole("alert");
    expect(alert).toHaveTextContent("The career changed before this could be saved.");
    expect(commandsCalled()).toContain(command);
    // The refused write changed nothing on screen: the setup is the one the backend still holds.
    expect(screen.getByRole("button", { name: "Choose tactic" })).toHaveTextContent(
      "Balanced Control",
    );
  });

  /**
   * Given an alert for a refused preset
   * When the backend accepts the next write
   * Then the alert is gone and the screen shows what the backend holds.
   */
  it("clears the alert once a later write saves", async () => {
    refused = new Set(["apply_tactic_preset"]);
    fakeBackend();
    render(<ControlledTacticsTab />);
    fireEvent.click(screen.getByRole("button", { name: "Choose tactic" }));
    fireEvent.click(screen.getByRole("option", { name: /High Press/ }));
    await screen.findByRole("alert");

    refused.clear();
    fireEvent.click(screen.getByRole("button", { name: "Choose tactic" }));
    fireEvent.click(screen.getByRole("option", { name: /High Press/ }));

    await waitFor(() => expect(screen.queryByRole("alert")).toBeNull());
    expect(screen.getByRole("button", { name: "Choose tactic" })).toHaveTextContent("High Press");
  });

  /**
   * Given an alert for a refused preset
   * When the manager saves the setup as a tactic and the save accepts it
   * Then the alert is gone: a write that saved, from the tactic library too, clears it.
   */
  it("clears the alert once a custom tactic save is accepted", async () => {
    refused = new Set(["apply_tactic_preset"]);
    fakeBackend();
    render(<ControlledTacticsTab />);
    fireEvent.click(screen.getByRole("button", { name: "Choose tactic" }));
    fireEvent.click(screen.getByRole("option", { name: /High Press/ }));
    await screen.findByRole("alert");

    fireEvent.click(screen.getByRole("button", { name: "Save as tactic" }));

    await waitFor(() => expect(screen.queryByRole("alert")).toBeNull());
    expect(commandsCalled()).toContain("save_current_tactic");
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
        careerToken: 0,
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
      // The backend applies the saved tactic whole (#1189); no preset, so no blueprint reset.
      await waitFor(() =>
        expect(mockedInvoke).toHaveBeenCalledWith("apply_custom_tactic", { careerToken: 0, id }),
      );
      expect(commandsCalled()).not.toContain("apply_tactic_preset");
      await waitFor(() =>
        expect(screen.getByRole("button", { name: "Choose tactic" })).toHaveTextContent(
          "Saved custom",
        ),
      );
      fireEvent.click(screen.getByRole("combobox", { name: "Play style" }));
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

    fireEvent.click(screen.getByRole("combobox", { name: "Play style" }));
    fireEvent.click(screen.getByRole("option", { name: /Defensive/ }));

    await waitFor(() => {
      expect(mockedInvoke).toHaveBeenCalledWith("set_play_style", {
        careerToken: 0,
        playStyle: "Defensive",
      });
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
      expect(mockedInvoke).toHaveBeenCalledWith("set_formation", {
        careerToken: 0,
        formation: "5-3-2",
      }),
    );
    mockedInvoke.mockClear();

    openActionsMenu();
    fireEvent.click(screen.getByRole("menuitem", { name: "Reset to defaults" }));

    await waitFor(() =>
      expect(mockedInvoke).toHaveBeenCalledWith("apply_tactic_preset", {
        careerToken: 0,
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
    openActionsMenu();
    expect(screen.getByRole("menuitem", { name: "Reset to defaults" })).toBeDisabled();
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
    openActionsMenu();
    const reset = screen.getByRole("menuitem", { name: "Reset to defaults" });
    expect(reset).toBeEnabled();

    fireEvent.click(reset);

    await waitFor(() =>
      expect(mockedInvoke).toHaveBeenCalledWith("apply_tactic_preset", {
        careerToken: 0,
        formation: "4-4-2",
        playStyle: "Balanced",
      }),
    );
    await waitFor(() => expect(screen.getByText("Synced")).toBeInTheDocument());
    openActionsMenu();
    expect(screen.getByRole("menuitem", { name: "Reset to defaults" })).toBeDisabled();
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
      expect(mockedInvoke).toHaveBeenCalledWith("set_formation", {
        careerToken: 0,
        formation: "5-3-2",
      }),
    );
    mockedInvoke.mockClear();

    openActionsMenu();
    fireEvent.click(screen.getByRole("menuitem", { name: "Reset to defaults" }));

    // Reset puts the saved tactic back, through the backend, as it was saved.
    await waitFor(() =>
      expect(mockedInvoke).toHaveBeenCalledWith("apply_custom_tactic", {
        careerToken: 0,
        id: "custom:saved",
      }),
    );
    await waitFor(() =>
      expect(screen.getByRole("combobox", { name: "Formation" })).toHaveTextContent("4-4-2"),
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
