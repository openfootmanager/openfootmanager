import { act, renderHook } from "@testing-library/react";
import { invoke } from "@tauri-apps/api/core";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import type { TeamData } from "../../store/gameStore";
import { useBlueprintModified } from "./useBlueprintModified";

vi.mock("@tauri-apps/api/core", () => ({ invoke: vi.fn() }));

const mockedInvoke = vi.mocked(invoke);
const team = { id: "t1", tactics_phase: {} } as unknown as TeamData;
const asks = () => mockedInvoke.mock.calls.filter(([c]) => c === "get_tactics_blueprint_status");

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
   * When the retries run out
   * Then it stops asking (the first ask and three retries), and keeps reading as modified.
   */
  it("gives up after three retries", async () => {
    mockedInvoke.mockRejectedValue(new Error("down"));

    const { result } = renderHook(() => useBlueprintModified("Balanced", team));
    await settle();
    await act(async () => {
      await vi.advanceTimersByTimeAsync(60_000);
    });

    expect(asks()).toHaveLength(4);
    expect(result.current).toBe(true);
  });

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
