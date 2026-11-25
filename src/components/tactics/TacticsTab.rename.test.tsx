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
  openActionsMenu,
  resetHarness,
  seedSavedTactics,
} from "./TacticsTab.harness";

/**
 * Renaming a saved tactic from the tactic bar's actions menu: a small dialog asks for the name, the
 * save renames it (`rename_custom_tactic`) and the picker shows the new name; a refusal is shown in
 * the dialog. Real i18n, and the Tactics tab's shared harness.
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

/** The saved tactic chosen from the picker, so it is the active one. */
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
  // Applying a saved tactic is asynchronous, and Rename is off until it is the active one.
  await waitFor(() =>
    expect(screen.getByRole("button", { name: "Choose tactic" })).toHaveTextContent("Saved custom"),
  );
}

describe("TacticsTab rename", { timeout: TACTICS_TAB_CASE_TIMEOUT_MS }, () => {
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
   * Given a saved tactic active
   * When the manager chooses Rename…, types a new name and presses Enter
   * Then the save is asked to rename it under the career token, the dialog closes and the picker
   *      shows the new name.
   */
  it("renames the saved tactic and shows the new name in the picker", async () => {
    await chooseTheSavedTactic();
    openActionsMenu();
    fireEvent.click(screen.getByRole("menuitem", { name: "Rename…" }));
    const dialog = await screen.findByRole("dialog", { name: "Rename tactic" });
    const field = within(dialog).getByRole("textbox", { name: "Tactic name" });

    fireEvent.change(field, { target: { value: "Away" } });
    fireEvent.submit(field.closest("form") as HTMLFormElement);

    await waitFor(() =>
      expect(mockedInvoke).toHaveBeenCalledWith("rename_custom_tactic", {
        careerToken: 0,
        id: "custom:saved",
        name: "Away",
      }),
    );
    await waitFor(() => expect(screen.queryByRole("dialog")).toBeNull());
    expect(screen.getByRole("button", { name: "Choose tactic" })).toHaveTextContent("Away");
  });

  /**
   * Given a saved tactic active and a save that refuses the rename
   * When the manager renames it
   * Then the reason is shown in the dialog, which stays open, and the picker keeps the old name.
   */
  it("shows the save's reason in the dialog when it refuses the name", async () => {
    await chooseTheSavedTactic();
    knobs.refused = new Set(["rename_custom_tactic"]);
    openActionsMenu();
    fireEvent.click(screen.getByRole("menuitem", { name: "Rename…" }));
    const dialog = await screen.findByRole("dialog", { name: "Rename tactic" });
    const field = within(dialog).getByRole("textbox", { name: "Tactic name" });

    fireEvent.change(field, { target: { value: "Away" } });
    fireEvent.submit(field.closest("form") as HTMLFormElement);

    expect(await within(dialog).findByRole("alert")).toHaveTextContent(
      "The career changed before this could be saved.",
    );
    expect(screen.getByRole("dialog", { name: "Rename tactic" })).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Choose tactic" })).toHaveTextContent("Saved custom");
  });

  /**
   * Given a preset active (nothing saved to rename)
   * When the actions menu is opened
   * Then Rename… is there but off, and no rename is asked of the save.
   */
  it("keeps Rename… off for a preset", async () => {
    fakeBackend();
    render(<ControlledTacticsTab />);
    fireEvent.click(screen.getByRole("button", { name: "Choose tactic" }));
    fireEvent.click(screen.getByRole("option", { name: /High Press/ }));
    await waitFor(() => expect(commandsCalled()).toContain("apply_tactic_preset"));

    openActionsMenu();

    expect(screen.getByRole("menuitem", { name: "Rename…" })).toBeDisabled();
    expect(commandsCalled()).not.toContain("rename_custom_tactic");
  });
});
