import { invoke } from "@tauri-apps/api/core";
import { beforeEach, describe, expect, it, vi } from "vitest";

import type { GameStateData } from "../store/gameStore";
import { useGameStore } from "../store/gameStore";
import { invokeForCareer } from "./careerInvoke";

vi.mock("@tauri-apps/api/core", () => ({ invoke: vi.fn() }));

const mockedInvoke = vi.mocked(invoke);

describe("invokeForCareer", () => {
  beforeEach(() => {
    mockedInvoke.mockReset();
    mockedInvoke.mockResolvedValue(undefined);
    useGameStore.setState({ gameState: null });
  });

  /**
   * Given the store shows a game installed as career 42
   * When a screen-write command is called
   * Then the call carries careerToken 42 beside its own arguments.
   */
  it("attaches the token of the career the screen shows", async () => {
    useGameStore.setState({ gameState: { career_token: 42 } as unknown as GameStateData });

    await invokeForCareer("advance_time_with_mode", { mode: "delegate" });

    expect(mockedInvoke).toHaveBeenCalledWith("advance_time_with_mode", {
      mode: "delegate",
      careerToken: 42,
    });
  });

  /**
   * Given a step asked in career 7 that runs after the screen shows career 8
   * When it passes the token it captured
   * Then that token is sent, not the store's current one.
   */
  it("sends the token it was given over the store's", async () => {
    useGameStore.setState({ gameState: { career_token: 8 } as unknown as GameStateData });

    await invokeForCareer("save_custom_tactic", { tactic: {} }, 7);

    expect(mockedInvoke).toHaveBeenCalledWith("save_custom_tactic", { tactic: {}, careerToken: 7 });
  });

  /**
   * Given no game in the store
   * When a command is called
   * Then it sends 0, a token no career carries, so the backend refuses it.
   */
  it("sends 0 when there is no game, which no career owns", async () => {
    await invokeForCareer("advance_one_day");

    expect(mockedInvoke).toHaveBeenCalledWith("advance_one_day", { careerToken: 0 });
  });
});
