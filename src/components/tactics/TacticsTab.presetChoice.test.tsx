import { fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import { beforeAll, beforeEach, describe, expect, it, vi } from "vitest";
import { i18nReady } from "../../i18n";
import { resetTacticsLineupCache } from "../../hooks/useTacticsLineupProjection";
import type { PlayerData } from "../../store/gameStore";
import {
  commandsCalled,
  ControlledTacticsTab,
  fakeBackend,
  knobs,
  mockedInvoke,
  openPhaseBlueprint,
  resetHarness,
} from "./TacticsTab.harness";

/**
 * Picking a preset sets its formation, play style and its own team instructions, so it asks first
 * when that would replace instructions the manager tuned (the backend answers whether the manager's
 * instructions differ from the preset's style's blueprint): Replace my instructions, Keep mine, or
 * neither. Real i18n, and the Tactics tab's shared harness.
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

// Every case mounts the whole TacticsTab with the real i18n; see TacticsTab.preset.test.tsx.
const TACTICS_TAB_CASE_TIMEOUT_MS = 20_000;

describe("TacticsTab preset question", { timeout: TACTICS_TAB_CASE_TIMEOUT_MS }, () => {
  beforeAll(async () => {
    await i18nReady;
  });

  beforeEach(() => {
    localStorage.clear();
    resetTacticsLineupCache();
    mockedInvoke.mockReset();
    resetHarness();
  });

  /** Tunes the width dial, so the manager's instructions differ from every preset's blueprint. */
  async function tuneAnInstruction() {
    openPhaseBlueprint();
    const widthRow = screen.getByText("Width").parentElement as HTMLElement;
    fireEvent.click(within(widthRow).getByRole("combobox"));
    fireEvent.click(screen.getByRole("option", { name: "Wide" }));
    await waitFor(() => expect(screen.getByText("Unsaved changes")).toBeInTheDocument());
  }

  /** Picks the High Press preset from the tactic picker. */
  function pickHighPress() {
    fireEvent.click(screen.getByRole("button", { name: "Choose tactic" }));
    fireEvent.click(screen.getByRole("option", { name: /High Press/ }));
  }

  /**
   * Given the manager's instructions already equal the preset's blueprint
   * When they pick the preset
   * Then the backend was asked about the preset's play style, and it applies without a question.
   */
  it("applies a preset without asking when none of the manager's instructions would be replaced", async () => {
    fakeBackend();
    render(<ControlledTacticsTab />);

    pickHighPress();

    await waitFor(() => expect(commandsCalled()).toContain("apply_tactic_preset"));
    expect(mockedInvoke).toHaveBeenCalledWith("get_tactics_blueprint_status", {
      playStyle: "HighPress",
    });
    expect(screen.queryByRole("dialog")).toBeNull();
  });

  /**
   * Given instructions the manager tuned
   * When they pick a preset
   * Then a dialog asks, and nothing is applied until they answer; "Replace my instructions" applies
   *      the whole preset.
   */
  it("asks before a preset replaces tuned instructions, and Replace applies the whole preset", async () => {
    fakeBackend();
    render(<ControlledTacticsTab />);
    await tuneAnInstruction();
    mockedInvoke.mockClear();

    pickHighPress();

    const dialog = await screen.findByRole("dialog", { name: "Replace your team instructions?" });
    expect(commandsCalled()).not.toContain("apply_tactic_preset");
    fireEvent.click(within(dialog).getByRole("button", { name: "Replace my instructions" }));

    await waitFor(() =>
      expect(mockedInvoke).toHaveBeenCalledWith("apply_tactic_preset", {
        careerToken: 0,
        formation: "3-4-3",
        playStyle: "HighPress",
      }),
    );
    expect(screen.queryByRole("dialog")).toBeNull();
  });

  /**
   * Given the question about tuned instructions
   * When the manager answers "Keep mine"
   * Then the preset's formation and play style are set, the preset command is not used (it would
   *      replace the instructions), and the dials the manager tuned are still tuned.
   */
  it("takes only the formation and play style when the manager keeps their instructions", async () => {
    fakeBackend();
    render(<ControlledTacticsTab />);
    await tuneAnInstruction();
    mockedInvoke.mockClear();

    pickHighPress();
    fireEvent.click(await screen.findByRole("button", { name: "Keep mine" }));

    await waitFor(() => expect(commandsCalled()).toContain("set_play_style"));
    expect(mockedInvoke).toHaveBeenCalledWith("set_formation", {
      careerToken: 0,
      formation: "3-4-3",
    });
    expect(mockedInvoke).toHaveBeenCalledWith("set_play_style", {
      careerToken: 0,
      playStyle: "HighPress",
    });
    expect(commandsCalled()).not.toContain("apply_tactic_preset");
    expect(screen.queryByRole("dialog")).toBeNull();
    expect(await screen.findByText("Unsaved changes")).toBeInTheDocument();
  });

  /**
   * Given the question about tuned instructions
   * When the manager cancels, and, in another round, presses Escape
   * Then nothing is applied either time and the tactic is unchanged.
   */
  it.each([
    ["Cancel", () => fireEvent.click(screen.getByRole("button", { name: "Cancel" }))],
    ["Escape", () => fireEvent.keyDown(window, { key: "Escape" })],
  ])("applies nothing when the question is answered with %s", async (_name, answer) => {
    fakeBackend();
    render(<ControlledTacticsTab />);
    await tuneAnInstruction();
    mockedInvoke.mockClear();

    pickHighPress();
    await screen.findByRole("dialog", { name: "Replace your team instructions?" });
    answer();

    await waitFor(() => expect(screen.queryByRole("dialog")).toBeNull());
    expect(commandsCalled()).not.toContain("apply_tactic_preset");
    expect(commandsCalled()).not.toContain("set_play_style");
  });

  /**
   * Given a backend that cannot say whether the preset would replace the manager's instructions
   * When they pick the preset
   * Then it asks: nothing of theirs is replaced on a guess.
   */
  it("asks when the backend cannot say whether the preset would replace instructions", async () => {
    knobs.blueprintStatusFailsForStyle = true;
    fakeBackend();
    render(<ControlledTacticsTab />);

    pickHighPress();

    expect(
      await screen.findByRole("dialog", { name: "Replace your team instructions?" }),
    ).toBeInTheDocument();
    expect(commandsCalled()).not.toContain("apply_tactic_preset");
  });
});
