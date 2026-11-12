import { act, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { afterEach, beforeAll, beforeEach, describe, expect, it, vi } from "vitest";
import { i18nReady } from "../../i18n";
import { resetTacticsLineupCache } from "../../hooks/useTacticsLineupProjection";
import type { PlayerData } from "../../store/gameStore";
import {
  commandsCalled,
  ControlledTacticsTab,
  fakeBackend,
  knobs,
  mockedInvoke,
  resetHarness,
  seedSavedTactics,
} from "./TacticsTab.harness";

/**
 * The tactic bar's badge for a saved tactic reads the backend's own comparison of the team's
 * tactic with the saved one (`get_custom_tactic_differences`): the eleven, the dials, the roles and
 * the takers are the backend's to compare, not the screen's. Real i18n, and the Tactics tab's
 * shared harness (a controlled parent re-rendering from the faked backend's game).
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

/** Holds the backend's answer to the comparison until the returned function is called. */
function holdTheComparison(): () => void {
  let release!: () => void;
  knobs.savedTacticDifferencesGate = new Promise<void>((resolve) => {
    release = resolve;
  });
  return release;
}

/** A saved tactic whose formation, style and name are the team's own, chosen from the picker. */
async function chooseTheSavedTactic() {
  seedSavedTactics([
    {
      id: "custom:saved",
      name: "Saved custom",
      description: "",
      formation: "4-4-2",
      playStyle: "Balanced",
    },
  ]);
  fakeBackend();
  render(<ControlledTacticsTab />);
  fireEvent.click(screen.getByRole("button", { name: "Choose tactic" }));
  fireEvent.click(await screen.findByRole("option", { name: /Saved custom/ }));
}

describe("TacticsTab saved tactic comparison", { timeout: TACTICS_TAB_CASE_TIMEOUT_MS }, () => {
  beforeAll(async () => {
    await i18nReady;
  });

  beforeEach(() => {
    localStorage.clear();
    resetTacticsLineupCache();
    mockedInvoke.mockReset();
    resetHarness();
  });

  afterEach(() => {
    vi.unstubAllGlobals();
  });

  /**
   * Given a saved tactic chosen, and the backend's answer to how the team's tactic differs from it
   *       still pending
   * When the screen shows the badge, and then the backend answers that nothing differs
   * Then while pending it does not read Synced and Update tactic is on (a pending question is not a
   *      clean answer); once answered it reads Synced and Update is off.
   */
  it("reads Unsaved changes while the comparison is pending, and Synced once nothing differs", async () => {
    const answer = holdTheComparison();
    await chooseTheSavedTactic();

    expect(await screen.findByText("Unsaved changes")).toBeInTheDocument();
    expect(screen.queryByText("Synced")).toBeNull();
    expect(screen.getByRole("button", { name: "Update tactic" })).toBeEnabled();

    answer();

    expect(await screen.findByText("Synced")).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Update tactic" })).toBeDisabled();
  });

  /**
   * Given a saved tactic whose shape, style and name match the team's, and the comparison held
   * When the backend answers that the team's eleven differs from what the tactic holds
   * Then, the answer having landed, the badge still reads Unsaved changes with Update tactic on:
   *      it reads the whole tactic, not only its formation, style and name.
   */
  it("keeps Unsaved changes once the backend says the eleven differs from the saved tactic", async () => {
    knobs.savedTacticDifferences.eleven = true;
    const answer = holdTheComparison();
    await chooseTheSavedTactic();
    expect(await screen.findByText("Unsaved changes")).toBeInTheDocument();

    answer();

    await waitFor(() => expect(knobs.savedTacticDifferencesAnswered).toBe(1));
    expect(screen.getByText("Unsaved changes")).toBeInTheDocument();
    expect(screen.queryByText("Synced")).toBeNull();
    expect(screen.getByRole("button", { name: "Update tactic" })).toBeEnabled();
  });

  /**
   * Given a saved tactic and a backend that cannot say how the team's tactic differs from it
   * When the saved tactic is chosen and the failure has landed
   * Then the badge does not call it synced: it reads "Unsaved changes" and Update tactic stays on,
   *      so a failed question cannot switch saving off.
   */
  it("does not call a saved tactic synced when the backend cannot say how it differs", async () => {
    knobs.savedTacticDifferencesFail = true;
    await chooseTheSavedTactic();
    await waitFor(() => expect(commandsCalled()).toContain("get_custom_tactic_differences"));

    await act(async () => {
      // Let the rejection reach the hook.
      await new Promise((resolve) => setTimeout(resolve, 0));
    });

    expect(await screen.findByText("Unsaved changes")).toBeInTheDocument();
    expect(screen.queryByText("Synced")).toBeNull();
    expect(screen.getByRole("button", { name: "Update tactic" })).toBeEnabled();
  });
});
