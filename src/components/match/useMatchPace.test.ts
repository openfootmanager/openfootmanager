import { act, renderHook } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";

import { useMatchPace } from "./useMatchPace";

describe("useMatchPace", () => {
  /**
   * Given a match starting at fast
   * When it is paused and then toggled again
   * Then it stops, keeps fast as the pace it resumes at, and plays at fast again.
   */
  it("pauses and resumes at the pace it was playing", () => {
    const { result } = renderHook(() => useMatchPace("fast"));
    expect(result.current).toMatchObject({ isRunning: true, pace: "fast" });

    act(() => result.current.togglePause());
    expect(result.current).toMatchObject({ isRunning: false, pace: "fast" });

    act(() => result.current.togglePause());
    expect(result.current).toMatchObject({ isRunning: true, pace: "fast" });
  });

  /**
   * Given a paused match
   * When a speed is chosen
   * Then it plays at that speed, and a pace worth keeping is reported for the second half.
   */
  it("plays at a chosen speed and reports a pace worth keeping", () => {
    const onPreferredSpeedChange = vi.fn();
    const { result } = renderHook(() => useMatchPace("normal", onPreferredSpeedChange));
    act(() => result.current.pause());

    act(() => result.current.selectPace("slow"));
    expect(result.current).toMatchObject({ isRunning: true, pace: "slow" });
    expect(onPreferredSpeedChange).toHaveBeenCalledWith("slow");
  });

  /**
   * Given a match playing at normal
   * When Max is chosen
   * Then it plays at Max, but Max is not reported, so the second half doesn't skip to its end.
   */
  it("does not keep Max for the second half", () => {
    const onPreferredSpeedChange = vi.fn();
    const { result } = renderHook(() => useMatchPace("normal", onPreferredSpeedChange));

    act(() => result.current.selectPace("instant"));
    expect(result.current).toMatchObject({ isRunning: true, pace: "instant" });
    expect(onPreferredSpeedChange).not.toHaveBeenCalled();
  });
});
