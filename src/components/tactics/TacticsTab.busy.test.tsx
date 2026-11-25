import { act, fireEvent, render, screen, waitFor } from "@testing-library/react";
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
  openActionsMenu,
  resetHarness,
  seedSavedTactics,
} from "./TacticsTab.harness";

/**
 * While a tactic write is out (a saved tactic or a preset being applied, a delete the save has not
 * answered, a preset's lookup), the controls that could start another change to the team's tactic are
 * off and the bar says it is busy. Interleaving two such changes was the source of five rounds of
 * review findings on #1447, each fixed by tracking one more value; not offering the second change
 * closes the class. Real i18n and the Tactics tab's shared harness.
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

const SAVED = {
  id: "custom:saved",
  name: "Saved custom",
  description: "",
  formation: "4-3-3",
  playStyle: "Attacking",
};

/** A gate a test opens by hand. */
function gate(): { held: Promise<void>; release: () => void } {
  let release!: () => void;
  const held = new Promise<void>((resolve) => {
    release = resolve;
  });
  return { held, release };
}

/** The controls that start another change to the team's tactic, and the bar's own busy state. */
function expectBar(busy: boolean) {
  const enabled = !busy;
  const bar = screen.getByRole("group", { name: "Tactic" });
  expect(bar).toHaveAttribute("aria-busy", String(busy));
  for (const name of ["Choose tactic", "Tactic actions"]) {
    const control = screen.getByRole("button", { name });
    if (enabled) expect(control).toBeEnabled();
    else expect(control).toBeDisabled();
  }
  for (const name of ["Formation", "Play style"]) {
    const control = screen.getByRole("combobox", { name });
    if (enabled) expect(control).toBeEnabled();
    else expect(control).toBeDisabled();
  }
}

describe("TacticsTab while a tactic write is out", { timeout: TACTICS_TAB_CASE_TIMEOUT_MS }, () => {
  beforeAll(async () => {
    await i18nReady;
  });

  beforeEach(() => {
    localStorage.clear();
    resetTacticsLineupCache();
    mockedInvoke.mockReset();
    resetHarness();
  });

  /**
   * Given a saved tactic chosen, its apply held by the backend
   * When the bar is looked at, and then the apply lands
   * Then every control that could change the tactic is off and the bar is busy, and all are on again
   *      once it has landed: a second change cannot interleave with the first.
   */
  it("turns the controls off while a saved tactic is being applied", async () => {
    seedSavedTactics([SAVED]);
    const apply = gate();
    knobs.applyCustomTacticGate = apply.held;
    fakeBackend();
    render(<ControlledTacticsTab />);
    expectBar(false);
    fireEvent.click(screen.getByRole("button", { name: "Choose tactic" }));
    fireEvent.click(await screen.findByRole("option", { name: /Saved custom/ }));
    await waitFor(() => expect(commandsCalled()).toContain("apply_custom_tactic"));

    expectBar(true);
    await act(async () => apply.release());

    await waitFor(() => expectBar(false));
  });

  /**
   * Given a saved tactic active and a delete the save has not answered
   * When the bar is looked at, and then the save answers
   * Then every control is off and the bar busy while it is out (Reset and the saved-tactic apply
   *      cannot act on a tactic that is going), and all are on again after.
   */
  it("turns the controls off while a delete is waiting for the save", async () => {
    seedSavedTactics([SAVED]);
    fakeBackend();
    render(<ControlledTacticsTab />);
    fireEvent.click(screen.getByRole("button", { name: "Choose tactic" }));
    fireEvent.click(await screen.findByRole("option", { name: /Saved custom/ }));
    await waitFor(() => expect(commandsCalled()).toContain("apply_custom_tactic"));
    await waitFor(() => expectBar(false));
    const deleting = gate();
    knobs.deleteCustomTacticGate = deleting.held;

    openActionsMenu();
    fireEvent.click(screen.getByRole("menuitem", { name: "Delete" }));
    fireEvent.click(screen.getByRole("button", { name: "Delete" }));
    await waitFor(() => expect(commandsCalled()).toContain("delete_custom_tactic"));

    expectBar(true);
    await act(async () => deleting.release());

    await waitFor(() => expectBar(false));
  });

  /**
   * Given a preset chosen whose lookup (does it replace the instructions?) the backend holds
   * When the bar is looked at, and then the lookup answers
   * Then every control is off and the bar busy while it is out, and all are on again once the preset
   *      has been applied.
   */
  it("turns the controls off while a preset's lookup is out", async () => {
    const lookup = gate();
    knobs.blueprintStatusGate = lookup.held;
    fakeBackend();
    render(<ControlledTacticsTab />);
    expectBar(false);
    fireEvent.click(screen.getByRole("button", { name: "Choose tactic" }));
    fireEvent.click(await screen.findByRole("option", { name: /Wing Play/ }));
    await waitFor(() => expect(commandsCalled()).toContain("get_tactics_blueprint_status"));

    expectBar(true);
    await act(async () => lookup.release());

    await waitFor(() => expect(commandsCalled()).toContain("apply_tactic_preset"));
    await waitFor(() => expectBar(false));
  });
});
