import { invoke } from "@tauri-apps/api/core";
import { beforeEach, describe, expect, it, vi } from "vitest";

import {
  applyCustomTactic,
  deleteCustomTactic,
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
