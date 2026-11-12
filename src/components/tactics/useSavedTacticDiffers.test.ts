import { act, renderHook, waitFor } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";

import { getCustomTacticDifferences } from "../../services/customTacticsService";
import type { TacticDifferences } from "../../store/generated/TacticDifferences";
import type { TeamData } from "../../store/gameStore";
import type { TacticsLibraryEntry } from "./TacticPicker";
import { useActiveSavedTacticDiffers, useSavedTacticDiffers } from "./useSavedTacticDiffers";

vi.mock("../../services/customTacticsService", () => ({ getCustomTacticDifferences: vi.fn() }));
const asked = vi.mocked(getCustomTacticDifferences);

const NOTHING: TacticDifferences = {
  formation: false,
  playStyle: false,
  instructions: false,
  eleven: false,
  roles: false,
  takers: false,
};

/** An answer the test lets through when it chooses. */
function held<T>() {
  let resolve!: (value: T) => void;
  let reject!: (reason: unknown) => void;
  const promise = new Promise<T>((res, rej) => {
    resolve = res;
    reject = rej;
  });
  return { promise, resolve, reject };
}

const library = [{}];

describe("useSavedTacticDiffers", () => {
  beforeEach(() => {
    asked.mockReset();
  });

  /**
   * Given a saved tactic and a club, and the backend's answer held back
   * When the hook is read before and after the answer says nothing differs
   * Then it says "differs" while pending and "same" once answered.
   */
  it("counts a pending question as differing, and a clean answer as the same", async () => {
    const answer = held<TacticDifferences>();
    asked.mockReturnValue(answer.promise);
    const { result } = renderHook(() => useSavedTacticDiffers("a", true, "team-1", library));
    expect(result.current).toBe(true);

    await act(async () => answer.resolve(NOTHING));

    expect(result.current).toBe(false);
    expect(asked).toHaveBeenCalledWith("a");
  });

  /**
   * Given a saved tactic and a club
   * When the backend answers that exactly one part differs (each of the six, in turn)
   * Then, once the answer has settled, it says "differs".
   */
  it.each(Object.keys(NOTHING) as (keyof TacticDifferences)[])(
    "says differs when only the %s differs",
    async (part) => {
      const answer = held<TacticDifferences>();
      asked.mockReturnValue(answer.promise);
      const { result } = renderHook(() => useSavedTacticDiffers("a", true, "team-1", library));

      await act(async () => answer.resolve({ ...NOTHING, [part]: true }));

      expect(result.current).toBe(true);
    },
  );

  /**
   * Given a saved tactic and a club
   * When the backend cannot answer (the request fails)
   * Then it says "differs": a failure is never read as the same.
   */
  it("says differs when the backend cannot answer", async () => {
    const answer = held<TacticDifferences>();
    asked.mockReturnValue(answer.promise);
    const { result } = renderHook(() => useSavedTacticDiffers("a", true, "team-1", library));

    await act(async () => answer.reject(new Error("no answer")));

    expect(result.current).toBe(true);
  });

  /**
   * Given a clean answer for one state of the team's tactic
   * When the team's tactic changes (a new question) and its answer is still pending
   * Then the earlier clean answer is not reused: it says "differs" until the new answer lands.
   */
  it("does not reuse an answer to an earlier question", async () => {
    asked.mockResolvedValueOnce(NOTHING);
    const { result, rerender } = renderHook(
      ({ key }) => useSavedTacticDiffers("a", true, key, library),
      { initialProps: { key: "team-1" } },
    );
    await waitFor(() => expect(result.current).toBe(false));
    const second = held<TacticDifferences>();
    asked.mockReturnValue(second.promise);

    rerender({ key: "team-2" });

    expect(result.current).toBe(true);
    await act(async () => second.resolve(NOTHING));
    expect(result.current).toBe(false);
  });

  /**
   * Given a question that is replaced by another, whose answer (the same) has already landed
   * When the first question's answer finally lands, saying a part differs
   * Then it is dropped: the answer to the latest question stands.
   */
  it("drops a late answer to an older question", async () => {
    const first = held<TacticDifferences>();
    asked.mockReturnValueOnce(first.promise).mockResolvedValueOnce(NOTHING);
    const { result, rerender } = renderHook(
      ({ key }) => useSavedTacticDiffers("a", true, key, library),
      { initialProps: { key: "team-1" } },
    );
    rerender({ key: "team-2" });
    await waitFor(() => expect(result.current).toBe(false));

    await act(async () => first.resolve({ ...NOTHING, eleven: true }));

    expect(result.current).toBe(false);
    expect(asked).toHaveBeenCalledTimes(2);
  });

  /**
   * Given no saved tactic is active, or no club
   * When the hook is read
   * Then it says "same" and asks nothing.
   */
  it.each([
    ["no saved tactic", null, true],
    ["no club", "a", false],
  ])("says the same and asks nothing with %s", (_name, savedId, hasClub) => {
    const { result } = renderHook(() => useSavedTacticDiffers(savedId, hasClub, "team-1", library));

    expect(result.current).toBe(false);
    expect(asked).not.toHaveBeenCalled();
  });

  describe("the team's tactic, read from the team", () => {
    const saved: TacticsLibraryEntry = {
      description: "",
      formation: "4-4-2",
      id: "a",
      name: "Saved",
      playStyle: "Balanced",
      sourcePresetName: null,
      type: "custom",
    };
    const savedTactics = [saved];
    const team = {
      formation: "4-4-2",
      play_style: "Balanced",
      tactics_phase: { width: "Normal" },
      starting_xi_ids: ["p1", "p2"],
      player_roles: { p1: "Standard" },
      match_roles: { captain: "p1" },
    } as unknown as TeamData;

    /**
     * Given a clean answer for the team's tactic as it is
     * When any one of the team's fields the tactic holds changes (each, in turn)
     * Then a new comparison is asked and, while it is pending, it says "differs": the field is part of
     *      the question, so an answer to the old tactic is not reused for the new one.
     */
    it.each([
      ["formation", { formation: "4-3-3" }],
      ["play style", { play_style: "Attacking" }],
      ["instructions", { tactics_phase: { width: "Wide" } }],
      ["starting eleven", { starting_xi_ids: ["p2", "p1"] }],
      ["player roles", { player_roles: { p1: "Stopper" } }],
      ["takers", { match_roles: { captain: "p2" } }],
    ])("asks again, pending as differing, when the %s changes", async (_name, change) => {
      asked.mockResolvedValueOnce(NOTHING);
      const { result, rerender } = renderHook(
        ({ current }) => useActiveSavedTacticDiffers(saved, savedTactics, current),
        { initialProps: { current: team } },
      );
      await waitFor(() => expect(result.current).toBe(false));
      expect(asked).toHaveBeenCalledTimes(1);
      const next = held<TacticDifferences>();
      asked.mockReturnValue(next.promise);

      rerender({ current: { ...team, ...change } as unknown as TeamData });

      expect(result.current).toBe(true);
      expect(asked).toHaveBeenCalledTimes(2);
      await act(async () => next.resolve(NOTHING));
      expect(result.current).toBe(false);
    });

    /**
     * Given a preset (a tactic nothing is saved for) is active
     * When the hook is read
     * Then it says "same" and asks nothing.
     */
    it("asks nothing for a tactic the save does not hold", () => {
      const { result } = renderHook(() =>
        useActiveSavedTacticDiffers(
          { ...saved, id: "preset:x", type: "preset" },
          savedTactics,
          team,
        ),
      );

      expect(result.current).toBe(false);
      expect(asked).not.toHaveBeenCalled();
    });
  });
});
