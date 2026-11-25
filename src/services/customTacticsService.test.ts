import { invoke } from "@tauri-apps/api/core";
import { beforeEach, describe, expect, it, vi } from "vitest";

import {
  applyCustomTactic,
  deleteCustomTactic,
  getCustomTacticDifferences,
  renameCustomTactic,
  legacyTacticsImportPending,
  listCustomTactics,
  markLegacyTacticsImported,
  saveCurrentTactic,
  saveCustomTactic,
} from "./customTacticsService";

vi.mock("@tauri-apps/api/core", () => ({ invoke: vi.fn() }));

const mockedInvoke = vi.mocked(invoke);

const tactic = {
  id: "a",
  name: "Mine",
  description: "",
  formation: "4-4-2",
  playStyle: "Balanced",
  sourcePresetName: null,
};

describe("customTacticsService", () => {
  beforeEach(() => {
    mockedInvoke.mockReset();
  });

  it("lists the saved tactics", async () => {
    mockedInvoke.mockResolvedValueOnce([tactic]);

    await expect(listCustomTactics()).resolves.toEqual([tactic]);
    expect(mockedInvoke).toHaveBeenCalledWith("list_custom_tactics");
  });

  /**
   * Given a saved tactic and a backend that says only the eleven differs
   * When the service asks how the team's tactic differs from it
   * Then it calls get_custom_tactic_differences with the tactic's id and hands the answer back whole.
   */
  it("asks the backend how the team's tactic differs from a saved one", async () => {
    const differences = {
      formation: false,
      playStyle: false,
      instructions: false,
      eleven: true,
      roles: false,
      takers: false,
    };
    mockedInvoke.mockResolvedValueOnce(differences);

    await expect(getCustomTacticDifferences("a")).resolves.toEqual(differences);
    expect(mockedInvoke).toHaveBeenCalledWith("get_custom_tactic_differences", { id: "a" });
  });

  /**
   * Given a saved tactic
   * When it is renamed under a career token
   * Then rename_custom_tactic is called with its id, the new name and the token, and the whole
   *      library comes back.
   */
  it("renames a tactic and returns the whole list", async () => {
    mockedInvoke.mockResolvedValueOnce([{ ...tactic, name: "Away" }]);

    await expect(renameCustomTactic("a", "Away", 7)).resolves.toEqual([
      { ...tactic, name: "Away" },
    ]);
    expect(mockedInvoke).toHaveBeenCalledWith("rename_custom_tactic", {
      id: "a",
      name: "Away",
      careerToken: 7,
    });
  });

  it("treats a reply that is not a list as no tactics", async () => {
    mockedInvoke.mockResolvedValueOnce({ manager: {} });

    await expect(listCustomTactics()).resolves.toEqual([]);
  });

  it("saves a tactic and returns the whole list", async () => {
    mockedInvoke.mockResolvedValueOnce([tactic]);

    await expect(saveCustomTactic(tactic, 7)).resolves.toEqual([tactic]);
    expect(mockedInvoke).toHaveBeenCalledWith("save_custom_tactic", { tactic, careerToken: 7 });
  });

  it("saves the team's whole tactic under the draft's name and returns the whole list", async () => {
    mockedInvoke.mockResolvedValueOnce([tactic]);

    await expect(saveCurrentTactic(tactic, 7)).resolves.toEqual([tactic]);
    expect(mockedInvoke).toHaveBeenCalledWith("save_current_tactic", { tactic, careerToken: 7 });
  });

  it("applies a saved tactic by id and returns the game", async () => {
    const game = { manager: { id: "m" } };
    mockedInvoke.mockResolvedValueOnce(game);

    await expect(applyCustomTactic("a", 7)).resolves.toBe(game);
    expect(mockedInvoke).toHaveBeenCalledWith("apply_custom_tactic", { id: "a", careerToken: 7 });
  });

  it("deletes a tactic by id", async () => {
    mockedInvoke.mockResolvedValueOnce([]);

    await expect(deleteCustomTactic("a", 7)).resolves.toEqual([]);
    expect(mockedInvoke).toHaveBeenCalledWith("delete_custom_tactic", { id: "a", careerToken: 7 });
  });

  it("asks whether the old browser library still has to be copied in", async () => {
    mockedInvoke.mockResolvedValueOnce(true);

    await expect(legacyTacticsImportPending()).resolves.toBe(true);
    expect(mockedInvoke).toHaveBeenCalledWith("legacy_tactics_import_pending");
  });

  it("marks the old browser library as copied in", async () => {
    mockedInvoke.mockResolvedValueOnce(undefined);

    await markLegacyTacticsImported(7);

    expect(mockedInvoke).toHaveBeenCalledWith("mark_legacy_tactics_imported", {
      careerToken: 7,
    });
  });
});
