import { fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import { StrictMode } from "react";
import { afterEach, beforeAll, beforeEach, describe, expect, it, vi } from "vitest";
import { i18nReady } from "../../i18n";
import { resetTacticsLineupCache } from "../../hooks/useTacticsLineupProjection";
import {
  commandsCalled,
  ControlledTacticsTab,
  fakeBackend,
  knobs,
  mockedInvoke,
  openActionsMenu,
  openPhaseBlueprint,
  resetHarness,
  seedSavedTactics,
} from "./TacticsTab.harness";
import type { PlayerData } from "../../store/gameStore";

/**
 * #365 — picking a tactic preset asks the backend for the whole preset
 * (formation, play style and its phase blueprint) in one command, while the
 * play-style dropdown on its own leaves the manager's dials alone.
 *
 * Real i18n hook, and a controlled parent that really re-renders from the
 * game the (faked) backend returns.
 */

vi.mock("../../hooks/useFamiliarity", async () => {
  const { cannedFamiliarityModule } = await import("../../test-utils/familiarity");
  const { familiarityState } = await import("../../test-utils/tacticsFamiliarityState");
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

/** The measuring stand-in `narrowLineupRegion` installs, to put back. */
let rectSpy: { mockRestore: () => void } | null = null;

// Every case mounts the whole TacticsTab with the real i18n and then drives several controls; that
// takes 2-5 s on a CI runner under load, and vitest's default 5 s cut-off turned the slowest cases
// into flaky failures. A generous per-describe limit keeps them honest without making them faster
// than the screen they exercise.
const TACTICS_TAB_CASE_TIMEOUT_MS = 20_000;

describe("TacticsTab preset selection", { timeout: TACTICS_TAB_CASE_TIMEOUT_MS }, () => {
  beforeAll(async () => {
    await i18nReady;
  });

  afterEach(() => {
    // Only what narrowLineupRegion put in place: the file's other mocks are set once, above.
    rectSpy?.mockRestore();
    rectSpy = null;
    vi.unstubAllGlobals();
  });

  beforeEach(() => {
    localStorage.clear();
    resetTacticsLineupCache();
    mockedInvoke.mockReset();
    resetHarness();
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

  /** Makes the lineup region 600px wide, so the pitch is behind its Pitch button. */
  function narrowLineupRegion() {
    rectSpy = vi
      .spyOn(HTMLElement.prototype, "getBoundingClientRect")
      .mockImplementation(
        () => ({ width: 600, height: 400, top: 0, left: 0, right: 600, bottom: 400 }) as DOMRect,
      );
    vi.stubGlobal(
      "ResizeObserver",
      class {
        observe(): void {}
        disconnect(): void {}
      },
    );
  }

  /** Opens the pitch panel of a narrow window and returns a pitch token's role control in it. */
  async function openPitchPanelRoleControl() {
    narrowLineupRegion();
    fakeBackend();
    render(<ControlledTacticsTab />);
    fireEvent.click(await screen.findByRole("button", { name: "Pitch" }));
    const panel = screen.getByRole("dialog", { name: "Pitch" });
    // The pitch's role controls are the panel's comboboxes: any one will do.
    return (await within(panel).findAllByRole("combobox"))[0];
  }

  /**
   * Given the pitch panel open in a narrow window, with the real pitch's role controls in it
   * When Enter and Space (which the control uses), then Tab and Escape are pressed on a role control
   *      (Escape last: it closes the panel and detaches the control)
   * Then Tab and Escape reach the window, where the dialog traps Tab and closes on Escape, and Enter
   *      and Space do not, or the token under the control would be activated too.
   */
  it("lets Tab and Escape from a pitch role control reach the dialog, but not Enter or Space", async () => {
    const control = await openPitchPanelRoleControl();
    const seen: string[] = [];
    const listener = (event: KeyboardEvent) => seen.push(event.key);
    window.addEventListener("keydown", listener);

    for (const key of [" ", "Enter", "Tab", "Escape"]) {
      fireEvent.keyDown(control, { key });
    }
    window.removeEventListener("keydown", listener);

    expect(seen).toEqual(["Tab", "Escape"]);
  });

  /**
   * Given the pitch panel open, and a role control focused whose list is closed
   * When Escape is pressed
   * Then the panel closes, as Escape does anywhere in a dialog.
   */
  it("closes the pitch panel on Escape from a closed role control", async () => {
    const control = await openPitchPanelRoleControl();
    control.focus();

    fireEvent.keyDown(control, { key: "Escape" });

    expect(screen.queryByRole("dialog", { name: "Pitch" })).not.toBeInTheDocument();
  });

  /**
   * Given the pitch panel open and the pointer over a token, so its summary is showing
   * When Escape is pressed on the token's role control, twice
   * Then the first dismisses the summary only (the token claims it), and the second closes the panel.
   */
  it("spends the first Escape on a token's summary and the second on the pitch panel", async () => {
    const control = await openPitchPanelRoleControl();
    fireEvent.mouseOver(control);
    control.focus();

    fireEvent.keyDown(control, { key: "Escape" });
    expect(screen.getByRole("dialog", { name: "Pitch" })).toBeInTheDocument();

    fireEvent.keyDown(control, { key: "Escape" });
    expect(screen.queryByRole("dialog", { name: "Pitch" })).not.toBeInTheDocument();
  });

  /**
   * Given the save has not yet answered the library's first load
   * When the manager picks a preset (whose Save is always on once the library is there)
   * Then Save stays disabled, and turns on once the load has answered.
   */
  it("keeps Save disabled until the library's first load has answered", async () => {
    let answerLoad!: () => void;
    knobs.libraryListGate = new Promise<void>((resolve) => {
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
    knobs.libraryListReleases = [];
    const releases = knobs.libraryListReleases;
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
    knobs.libraryListGate = new Promise<void>((resolve) => {
      answerLoad = resolve;
    });
    knobs.libraryListFailsOnce = true;
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
    knobs.refused = new Set([command]);
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
    knobs.refused = new Set(["apply_tactic_preset"]);
    fakeBackend();
    render(<ControlledTacticsTab />);
    fireEvent.click(screen.getByRole("button", { name: "Choose tactic" }));
    fireEvent.click(screen.getByRole("option", { name: /High Press/ }));
    await screen.findByRole("alert");

    knobs.refused.clear();
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
    knobs.refused = new Set(["apply_tactic_preset"]);
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
  it("disables Reset on an untouched preset", async () => {
    fakeBackend();
    render(<ControlledTacticsTab />);

    // The badge says Synced once the library has loaded (until then it says so is loading).
    expect(await screen.findByText("Synced")).toBeInTheDocument();
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
    expect(await screen.findByText("Synced")).toBeInTheDocument();

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
