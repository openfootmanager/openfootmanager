import { act, renderHook } from "@testing-library/react";
import { invoke } from "@tauri-apps/api/core";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import type { TeamData } from "../../store/gameStore";
import { useBlueprintModified } from "./useBlueprintModified";

vi.mock("@tauri-apps/api/core", () => ({ invoke: vi.fn() }));

const mockedInvoke = vi.mocked(invoke);
const team = { id: "t1", tactics_phase: {} } as unknown as TeamData;
const asks = () => mockedInvoke.mock.calls.filter(([c]) => c === "get_tactics_blueprint_status");

/** A promise settled by hand, so a test decides when an answer arrives. */
function deferred<T>() {
  let resolve: (value: T) => void = () => {};
  let reject: (reason: unknown) => void = () => {};
  const promise = new Promise<T>((res, rej) => {
    resolve = res;
    reject = rej;
  });
  return { promise, resolve, reject };
}

/** Lets the pending promise callbacks run without moving the fake clock. */
const settle = () => act(async () => {});

describe("useBlueprintModified (#1438)", () => {
  beforeEach(() => {
    vi.useFakeTimers();
    mockedInvoke.mockReset();
  });

  afterEach(() => {
    vi.useRealTimers();
  });

  /**
   * Given the backend's verdict on the dials failing (a transient refusal)
   * When the hook has asked
   * Then the dials read as differing from the blueprint, not as matching: a question the backend
   *      cannot answer must not read "no" (it would hide Reset on a tuned preset).
   */
  it("reads as modified while the backend cannot answer", async () => {
    mockedInvoke.mockRejectedValue(new Error("busy"));

    const { result } = renderHook(() => useBlueprintModified("Balanced", team));
    await settle();

    expect(result.current).toBe(true);
  });

  /**
   * Given a verdict that failed once and the backend then recovering
   * When the retry delay has passed
   * Then it is asked again for the same inputs, and the answer replaces the guess.
   */
  it("asks again after a failure, and takes the answer once it comes", async () => {
    mockedInvoke.mockRejectedValueOnce(new Error("busy")).mockResolvedValue({ modified: false });

    const { result } = renderHook(() => useBlueprintModified("Balanced", team));
    await settle();
    expect(result.current).toBe(true);
    expect(asks()).toHaveLength(1);

    await act(async () => {
      await vi.advanceTimersByTimeAsync(1000);
    });

    expect(asks()).toHaveLength(2);
    expect(result.current).toBe(false);
  });

  /**
   * Given a backend that keeps failing
   * When time passes
   * Then each retry comes exactly when its delay is up: nothing 1 ms before 1 s, 3 s and 9 s after
   *      the failure that scheduled it, exactly one call as each is crossed, and no further call after
   *      the fourth failure.
   */
  it("retries at 1 s, 3 s and 9 s, exactly, then stops", async () => {
    mockedInvoke.mockRejectedValue(new Error("down"));

    const { result } = renderHook(() => useBlueprintModified("Balanced", team));
    await settle();
    expect(asks()).toHaveLength(1);

    for (const [delay, callsAfter] of [
      [1_000, 2],
      [3_000, 3],
      [9_000, 4],
    ] as const) {
      await act(async () => {
        await vi.advanceTimersByTimeAsync(delay - 1);
      });
      expect(asks()).toHaveLength(callsAfter - 1);
      await act(async () => {
        await vi.advanceTimersByTimeAsync(1);
      });
      expect(asks()).toHaveLength(callsAfter);
    }

    await act(async () => {
      await vi.advanceTimersByTimeAsync(60_000);
    });
    expect(asks()).toHaveLength(4);
    expect(result.current).toBe(true);
  });

  /**
   * Given a retry already asked (its answer still out) and then the dials changing, and the newer
   *       question answered "no"
   * When the old retry answers the opposite, or fails
   * Then neither changes what the hook says, and neither starts a retry chain of its own: the old
   *      question's answer is not this question's.
   */
  it.each([
    ["answers the opposite", "resolve"],
    ["fails", "reject"],
  ] as const)(
    "ignores an old retry that %s after the question changed",
    async (_name, settleAs) => {
      const oldRetry = deferred<{ modified: boolean }>();
      let call = 0;
      mockedInvoke.mockImplementation(async () => {
        call += 1;
        if (call === 1) throw new Error("busy");
        if (call === 2) return oldRetry.promise;
        return { modified: false };
      });

      const { result, rerender } = renderHook(({ style }) => useBlueprintModified(style, team), {
        initialProps: { style: "Balanced" },
      });
      await settle();
      await act(async () => {
        await vi.advanceTimersByTimeAsync(1_000);
      });
      expect(asks()).toHaveLength(2);

      rerender({ style: "Defensive" });
      await settle();
      expect(asks()).toHaveLength(3);
      expect(result.current).toBe(false);

      await act(async () => {
        if (settleAs === "resolve") oldRetry.resolve({ modified: true });
        else oldRetry.reject(new Error("late failure"));
      });
      await act(async () => {
        await vi.advanceTimersByTimeAsync(60_000);
      });

      expect(result.current).toBe(false);
      expect(asks()).toHaveLength(3);
    },
  );

  /**
   * Given a retry waiting
   * When the dials change (a new question)
   * Then the old retry is cancelled and only the new question is asked.
   */
  it("drops a pending retry when the question changes", async () => {
    mockedInvoke.mockRejectedValueOnce(new Error("busy")).mockResolvedValue({ modified: true });

    const { rerender } = renderHook(({ style }) => useBlueprintModified(style, team), {
      initialProps: { style: "Balanced" },
    });
    await settle();
    rerender({ style: "Defensive" });
    await settle();
    const askedBefore = asks().length;
    await act(async () => {
      await vi.advanceTimersByTimeAsync(60_000);
    });

    expect(askedBefore).toBe(2);
    expect(asks()).toHaveLength(2);
  });

  /**
   * Given a club with an answer of "no"
   * When the hook has asked
   * Then it reads false (the ordinary case is unchanged).
   */
  it("reads an answer of no as no", async () => {
    mockedInvoke.mockResolvedValue({ modified: false });

    const { result } = renderHook(() => useBlueprintModified("Balanced", team));
    await settle();

    expect(result.current).toBe(false);
  });
});
