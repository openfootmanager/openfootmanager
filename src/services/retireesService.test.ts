import { beforeEach, describe, expect, it, vi } from "vitest";
import { invoke } from "@tauri-apps/api/core";

import { fetchHallOfFameLegends, fetchRetiredPlayer } from "./retireesService";

vi.mock("@tauri-apps/api/core", () => ({
  invoke: vi.fn(),
}));

const mockedInvoke = vi.mocked(invoke);

describe("retireesService", () => {
  beforeEach(() => {
    mockedInvoke.mockReset();
  });

  it("asks the backend for the Hall of Fame legends", async () => {
    mockedInvoke.mockResolvedValueOnce([]);

    await expect(fetchHallOfFameLegends()).resolves.toEqual([]);

    expect(mockedInvoke).toHaveBeenCalledWith("get_hall_of_fame_legends");
  });

  it("asks the backend for a retired player by id", async () => {
    const player = { id: "p1", retired: true };
    mockedInvoke.mockResolvedValueOnce(player);

    await expect(fetchRetiredPlayer("p1")).resolves.toBe(player);

    expect(mockedInvoke).toHaveBeenCalledWith("get_retired_player", { playerId: "p1" });
  });
});
