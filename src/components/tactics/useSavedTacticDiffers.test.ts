import { act, renderHook, waitFor } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";

import { getCustomTacticDifferences } from "../../services/customTacticsService";
import type { TacticDifferences } from "../../store/generated/TacticDifferences";
import { useSavedTacticDiffers } from "./useSavedTacticDiffers";

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
   * When the backend answers that only the takers differ
   * Then it says "differs".
   */
  it("says differs when any one part differs", async () => {
    asked.mockResolvedValue({ ...NOTHING, takers: true });
    const { result } = renderHook(() => useSavedTacticDiffers("a", true, "team-1", library));

    await waitFor(() => expect(asked).toHaveBeenCalled());
    await act(async () => {});

    expect(result.current).toBe(true);
  });

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
   * Given a question that is later replaced by another
   * When the first answer lands after the second question was asked
   * Then it is dropped: only the answer to the latest question counts.
   */
  it("drops a late answer to an older question", async () => {
    const first = held<TacticDifferences>();
    const second = held<TacticDifferences>();
    asked.mockReturnValueOnce(first.promise).mockReturnValueOnce(second.promise);
    const { result, rerender } = renderHook(
      ({ key }) => useSavedTacticDiffers("a", true, key, library),
      { initialProps: { key: "team-1" } },
    );
    rerender({ key: "team-2" });

    await act(async () => first.resolve(NOTHING));
    expect(result.current).toBe(true);
    await act(async () => second.resolve({ ...NOTHING, eleven: true }));

    expect(result.current).toBe(true);
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
});
