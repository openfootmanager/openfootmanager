import { act, fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import { beforeAll, beforeEach, describe, expect, it, vi } from "vitest";
import { i18nReady } from "../../i18n";
import { resetTacticsLineupCache } from "../../hooks/useTacticsLineupProjection";
import { type PlayerData, useGameStore } from "../../store/gameStore";
import TacticsTab from "./TacticsTab";
import {
  commandsCalled,
  currentGame,
  fakeBackend,
  knobs,
  mockedInvoke,
  openPhaseBlueprint,
  resetHarness,
  seedSavedTactics,
} from "./TacticsTab.harness";

/**
 * The bar turns only the controls that start another change of the tactic off while an apply is out;
 * the instructions, roles, set pieces and lineup stay on. A late reply to the apply must not undo an
 * edit made meanwhile: the game store orders the games it is given by career and revision (#1394), so
 * the older reply is dropped. This runs the Tactics tab over the real store to show it.
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

/** The Tactics tab as the dashboard mounts it: its game is the store's, its updates go to the store. */
function StoreBackedTab() {
  const game = useGameStore((state) => state.gameState);
  const setGameState = useGameStore((state) => state.setGameState);
  return game ? (
    <TacticsTab gameState={game} onSelectPlayer={vi.fn()} onGameUpdate={setGameState} />
  ) : null;
}

describe("TacticsTab over the game store", { timeout: TACTICS_TAB_CASE_TIMEOUT_MS }, () => {
  beforeAll(async () => {
    await i18nReady;
  });

  beforeEach(() => {
    localStorage.clear();
    resetTacticsLineupCache();
    mockedInvoke.mockReset();
    resetHarness();
    useGameStore.getState().clearGame();
  });

  /**
   * Given a saved tactic being applied, its reply held (the backend has made the write, revision 1),
   *       and the manager then tunes an instruction (revision 2, its reply taken at once)
   * When the apply's older reply arrives
   * Then the store keeps the later game: the tuned instruction is still there, at revision 2.
   */
  it("keeps an instruction edited while a saved tactic was applying when the apply's reply arrives late", async () => {
    seedSavedTactics([
      {
        id: "custom:saved",
        name: "Saved custom",
        description: "",
        formation: "4-3-3",
        playStyle: "Attacking",
      },
    ]);
    let release!: () => void;
    knobs.applyCustomTacticReplyGate = new Promise<void>((resolve) => {
      release = resolve;
    });
    fakeBackend();
    useGameStore.getState().setGameState(currentGame());
    render(<StoreBackedTab />);
    fireEvent.click(screen.getByRole("button", { name: "Choose tactic" }));
    fireEvent.click(await screen.findByRole("option", { name: /Saved custom/ }));
    await waitFor(() => expect(commandsCalled()).toContain("apply_custom_tactic"));

    openPhaseBlueprint();
    const widthRow = screen.getByText("Width").parentElement as HTMLElement;
    fireEvent.click(within(widthRow).getByRole("combobox"));
    fireEvent.click(screen.getByRole("option", { name: "Wide" }));
    await waitFor(() => expect(useGameStore.getState().gameState?.revision).toBe(2));
    const tuned = useGameStore.getState().gameState?.teams[0].tactics_phase;
    await act(async () => release());
    await waitFor(() =>
      expect(screen.getByRole("button", { name: "Choose tactic" })).toHaveTextContent(
        "Saved custom",
      ),
    );

    const held = useGameStore.getState().gameState;
    expect(held?.revision).toBe(2);
    expect(held?.teams[0].tactics_phase).toEqual(tuned);
  });
});
