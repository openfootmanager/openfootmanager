import { act, fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import { beforeAll, beforeEach, describe, expect, it, vi } from "vitest";
import { i18nReady } from "../../i18n";
import { resetTacticsLineupCache } from "../../hooks/useTacticsLineupProjection";
import { useGameStore, type PlayerData } from "../../store/gameStore";
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
   * Then the preset is applied in one write that keeps their instructions (no separate formation and
   *      style writes that could half-apply), and the dials the manager tuned are still tuned.
   */
  it("applies the preset keeping the manager's instructions when they answer Keep mine", async () => {
    fakeBackend();
    render(<ControlledTacticsTab />);
    await tuneAnInstruction();
    mockedInvoke.mockClear();

    pickHighPress();
    fireEvent.click(await screen.findByRole("button", { name: "Keep mine" }));

    await waitFor(() =>
      expect(mockedInvoke).toHaveBeenCalledWith("apply_tactic_preset", {
        careerToken: 0,
        formation: "3-4-3",
        keepInstructions: true,
        playStyle: "HighPress",
      }),
    );
    expect(commandsCalled()).not.toContain("set_formation");
    expect(commandsCalled()).not.toContain("set_play_style");
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
    // The dialog's keyboard handling and its initial focus are set in the effects that run just
    // after it appears: Escape pressed before that finds no handler, so wait for the focus.
    await waitFor(() => expect(screen.getByRole("button", { name: "Keep mine" })).toHaveFocus());
    answer();

    await waitFor(() => expect(screen.queryByRole("dialog")).toBeNull());
    expect(commandsCalled()).not.toContain("apply_tactic_preset");
    expect(commandsCalled()).not.toContain("set_play_style");
  });

  /**
   * Given a preset chosen and the backend's answer about it still pending
   * When the manager chooses another preset whose answer comes at once (nothing tuned to replace), and
   *      then the first answer arrives saying it would replace tuned instructions
   * Then the first is dropped: no dialog opens for it and it is not applied over the second.
   */
  it("drops the answer to a choice the manager has since replaced", async () => {
    fakeBackend();
    render(<ControlledTacticsTab />);
    await tuneAnInstruction();
    let release!: () => void;
    knobs.blueprintStatusGate = new Promise<void>((resolve) => {
      release = resolve;
    });
    mockedInvoke.mockClear();
    pickHighPress();
    await waitFor(() => expect(commandsCalled()).toContain("get_tactics_blueprint_status"));

    knobs.blueprintStatusGate = null;
    knobs.blueprintModified = false;
    fireEvent.click(screen.getByRole("button", { name: "Choose tactic" }));
    fireEvent.click(screen.getByRole("option", { name: /Balanced Control/ }));
    await waitFor(() => expect(commandsCalled()).toContain("apply_tactic_preset"));
    knobs.blueprintModified = true;
    // The first lookup answers now, after the second choice was applied: wait for that answer to
    // be given and for the screen to take it.
    await act(async () => {
      release();
      await waitFor(() => expect(knobs.blueprintStatusAnswered).toBe(2));
    });

    expect(screen.queryByRole("dialog")).toBeNull();
    expect(
      mockedInvoke.mock.calls.filter(([command]) => command === "apply_tactic_preset"),
    ).toHaveLength(1);
  });

  /**
   * Given the question open about tuned instructions
   * When the career changes (another game is loaded)
   * Then the question is withdrawn: an answer is never applied to a career it was not asked in.
   */
  it("withdraws the question when the career changes", async () => {
    fakeBackend();
    render(<ControlledTacticsTab />);
    await tuneAnInstruction();
    pickHighPress();
    await screen.findByRole("dialog", { name: "Replace your team instructions?" });

    act(() => {
      useGameStore.setState((state) => ({ careerEpoch: state.careerEpoch + 1 }));
    });

    await waitFor(() => expect(screen.queryByRole("dialog")).toBeNull());
  });

  /**
   * Given a preset chosen while the backend says nothing of the manager's would be replaced, and
   *       that answer still pending
   * When the manager commits an instruction edit before the answer arrives
   * Then the preset is applied keeping their instructions, so the edit is not wiped by a whole apply.
   */
  it("does not wipe an instruction edited while the preset's lookup is pending", async () => {
    fakeBackend();
    render(<ControlledTacticsTab />);
    let release!: () => void;
    knobs.blueprintStatusGate = new Promise<void>((resolve) => {
      release = resolve;
    });
    mockedInvoke.mockClear();
    pickHighPress();
    await waitFor(() => expect(commandsCalled()).toContain("get_tactics_blueprint_status"));

    await tuneAnInstruction();
    await act(async () => {
      release();
      await waitFor(() => expect(knobs.blueprintStatusAnswered).toBe(1));
    });

    await waitFor(() =>
      expect(mockedInvoke).toHaveBeenCalledWith("apply_tactic_preset", {
        careerToken: 0,
        formation: "3-4-3",
        keepInstructions: true,
        playStyle: "HighPress",
      }),
    );
    expect(await screen.findByText("Unsaved changes")).toBeInTheDocument();
  });

  /**
   * Given a preset chosen and its lookup pending
   * When the tab goes away and the career is replaced, and then the lookup answers
   * Then nothing is applied: the choice was another career's.
   */
  it("applies nothing from a lookup answered after the tab left and the career changed", async () => {
    fakeBackend();
    const { unmount } = render(<ControlledTacticsTab />);
    let release!: () => void;
    knobs.blueprintStatusGate = new Promise<void>((resolve) => {
      release = resolve;
    });
    mockedInvoke.mockClear();
    pickHighPress();
    await waitFor(() => expect(commandsCalled()).toContain("get_tactics_blueprint_status"));

    unmount();
    act(() => {
      useGameStore.setState((state) => ({ careerEpoch: state.careerEpoch + 1 }));
    });
    await act(async () => {
      release();
      await waitFor(() => expect(knobs.blueprintStatusAnswered).toBe(1));
    });

    expect(commandsCalled()).not.toContain("apply_tactic_preset");
  });

  /**
   * Given a preset chosen and its lookup pending
   * When the career is replaced while the tab stays mounted, and then the lookup answers
   * Then nothing is applied and no question opens for it.
   */
  it("applies nothing from a lookup answered after the career changed under the tab", async () => {
    fakeBackend();
    render(<ControlledTacticsTab />);
    let release!: () => void;
    knobs.blueprintStatusGate = new Promise<void>((resolve) => {
      release = resolve;
    });
    mockedInvoke.mockClear();
    pickHighPress();
    await waitFor(() => expect(commandsCalled()).toContain("get_tactics_blueprint_status"));

    act(() => {
      useGameStore.setState((state) => ({ careerEpoch: state.careerEpoch + 1 }));
    });
    await act(async () => {
      release();
      await waitFor(() => expect(knobs.blueprintStatusAnswered).toBe(1));
    });

    expect(commandsCalled()).not.toContain("apply_tactic_preset");
    expect(screen.queryByRole("dialog")).toBeNull();
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
