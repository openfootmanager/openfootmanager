import { beforeEach, describe, expect, it, vi } from "vitest";
import { invoke } from "@tauri-apps/api/core";

import en from "../i18n/locales/en.json";
import type { ReputationBand } from "../store/generated/ReputationBand";
import { getManagersPage } from "./managersService";

vi.mock("@tauri-apps/api/core", () => ({ invoke: vi.fn() }));
const mockedInvoke = vi.mocked(invoke);

beforeEach(() => {
  mockedInvoke.mockReset();
});

/** Every band the backend sends; a band added in Rust fails the type check here until it is listed. */
const EVERY_BAND: Record<ReputationBand, true> = {
  world_class: true,
  established: true,
  great: true,
  good: true,
  average: true,
  below_average: true,
  unknown: true,
};

describe("managersService", () => {
  /**
   * Given the directory filtered to employed managers
   * When it is asked for
   * Then the command is called with that status and the answer comes back as it is
   */
  it("asks the backend for the managers directory with its status", async () => {
    const answer = { managers: [], vacancies: [] };
    mockedInvoke.mockResolvedValueOnce(answer);

    await expect(getManagersPage("employed")).resolves.toEqual(answer);
    expect(mockedInvoke).toHaveBeenCalledWith("get_managers_page", { status: "employed" });
  });

  /**
   * Given no active game
   * When the directory is asked for
   * Then the backend's translated error reaches the caller
   */
  it("passes on the backend's error", async () => {
    mockedInvoke.mockRejectedValueOnce("be.error.noActiveGameSession");

    await expect(getManagersPage("all")).rejects.toBe("be.error.noActiveGameSession");
  });

  /**
   * Given every reputation band the backend can send
   * When the screen looks up its word
   * Then English has one for each (the other locales are held to English by the locale tests)
   */
  it("has a word for every reputation band", () => {
    const words: Record<string, string> = en.managersWorld.reputationBand;
    const missing = Object.keys(EVERY_BAND).filter((band) => !words[band]);

    expect(missing).toEqual([]);
  });
});
