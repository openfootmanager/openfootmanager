import { invoke } from "@tauri-apps/api/core";
import { beforeEach, describe, expect, it, vi } from "vitest";

import { applySubstitutions } from "./applySubstitutions";
import type { MatchSnapshot } from "./types";

vi.mock("@tauri-apps/api/core", () => ({ invoke: vi.fn() }));

const mockedInvoke = vi.mocked(invoke);
const snapshotAfter = (made: number) => ({ home_subs_made: made }) as unknown as MatchSnapshot;

describe("applySubstitutions", () => {
  beforeEach(() => {
    mockedInvoke.mockReset();
  });

  /**
   * Given two queued swaps
   * When they are applied
   * Then each goes to the engine in order, the screen hears about every resulting snapshot, and
   *      the batch reports success.
   */
  it("applies every swap in order and reports each snapshot", async () => {
    mockedInvoke.mockResolvedValueOnce(snapshotAfter(1)).mockResolvedValueOnce(snapshotAfter(2));
    const onSnapshot = vi.fn();

    const applied = await applySubstitutions(
      "Home",
      [
        { offId: "a", onId: "b" },
        { offId: "c", onId: "d" },
      ],
      onSnapshot,
    );

    expect(applied).toBe(true);
    expect(mockedInvoke.mock.calls).toEqual([
      [
        "apply_match_command",
        { command: { Substitute: { side: "Home", player_off_id: "a", player_on_id: "b" } } },
      ],
      [
        "apply_match_command",
        { command: { Substitute: { side: "Home", player_off_id: "c", player_on_id: "d" } } },
      ],
    ]);
    expect(onSnapshot.mock.calls.map(([snapshot]) => snapshot.home_subs_made)).toEqual([1, 2]);
  });

  /**
   * Given the engine refuses the second of three swaps
   * When the batch is applied
   * Then the first is kept and reported, the rest are not attempted, and the batch reports failure.
   */
  it("stops at the first refused swap and keeps what was applied", async () => {
    vi.spyOn(console, "error").mockImplementation(() => {});
    mockedInvoke.mockResolvedValueOnce(snapshotAfter(1)).mockRejectedValueOnce("be.error.nope");
    const onSnapshot = vi.fn();

    const applied = await applySubstitutions(
      "Away",
      [
        { offId: "a", onId: "b" },
        { offId: "c", onId: "d" },
        { offId: "e", onId: "f" },
      ],
      onSnapshot,
    );

    expect(applied).toBe(false);
    expect(mockedInvoke).toHaveBeenCalledTimes(2);
    expect(onSnapshot).toHaveBeenCalledTimes(1);
  });
});
