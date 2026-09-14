import { beforeEach, describe, expect, it, vi } from "vitest";
import { invoke } from "@tauri-apps/api/core";

import {
  setPlayerTrainingFocus,
  setTraining,
  setTrainingGroups,
  setTrainingSchedule,
} from "./trainingService";

vi.mock("@tauri-apps/api/core", () => ({
  invoke: vi.fn(),
}));

const mockedInvoke = vi.mocked(invoke);

describe("trainingService", () => {
  beforeEach(() => {
    mockedInvoke.mockReset();
  });

  it("calls the set training backend command", async () => {
    const response = { manager: { id: "manager-1" } };
    mockedInvoke.mockResolvedValueOnce(response);

    await expect(setTraining("Physical", "High")).resolves.toBe(response);
    expect(mockedInvoke).toHaveBeenCalledWith("set_training", {
      focus: "Physical",
      intensity: "High",
    });
  });

  it("calls the set training schedule backend command", async () => {
    const response = { manager: { id: "manager-1" } };
    mockedInvoke.mockResolvedValueOnce(response);

    await expect(setTrainingSchedule("Light")).resolves.toBe(response);
    expect(mockedInvoke).toHaveBeenCalledWith("set_training_schedule", {
      schedule: "Light",
    });
  });

  it("calls the set training groups backend command", async () => {
    const response = { manager: { id: "manager-1" } };
    const groups = [{ id: "grp-1", name: "Attack", focus: "Attacking", player_ids: ["player-1"] }];
    mockedInvoke.mockResolvedValueOnce(response);

    await expect(setTrainingGroups(groups)).resolves.toBe(response);
    expect(mockedInvoke).toHaveBeenCalledWith("set_training_groups", {
      groups,
    });
  });

  it("calls the set player training focus backend command", async () => {
    const response = { manager: { id: "manager-1" } };
    mockedInvoke.mockResolvedValueOnce(response);

    await expect(setPlayerTrainingFocus("player-1", null)).resolves.toBe(response);
    expect(mockedInvoke).toHaveBeenCalledWith("set_player_training_focus", {
      playerId: "player-1",
      focus: null,
    });
  });
});
