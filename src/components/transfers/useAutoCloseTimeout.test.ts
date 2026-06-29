import { renderHook, act } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import { useAutoCloseTimeout } from "./useAutoCloseTimeout";

describe("useAutoCloseTimeout", () => {
  beforeEach(() => {
    vi.useFakeTimers();
  });

  afterEach(() => {
    vi.useRealTimers();
  });

  it("runs the scheduled callback once after the delay", () => {
    const callback = vi.fn();
    const { result } = renderHook(() => useAutoCloseTimeout());

    act(() => {
      result.current.scheduleAutoClose(callback, 1500);
    });
    expect(callback).not.toHaveBeenCalled();

    act(() => {
      vi.advanceTimersByTime(1500);
    });
    expect(callback).toHaveBeenCalledTimes(1);
  });

  it("does not fire the callback when unmounted before the delay elapses", () => {
    const callback = vi.fn();
    const { result, unmount } = renderHook(() => useAutoCloseTimeout());

    act(() => {
      result.current.scheduleAutoClose(callback, 1500);
    });

    unmount();

    act(() => {
      vi.advanceTimersByTime(1500);
    });
    expect(callback).not.toHaveBeenCalled();
  });

  it("clears a pending callback explicitly via clearAutoClose", () => {
    const callback = vi.fn();
    const { result } = renderHook(() => useAutoCloseTimeout());

    act(() => {
      result.current.scheduleAutoClose(callback, 1500);
      result.current.clearAutoClose();
    });

    act(() => {
      vi.advanceTimersByTime(1500);
    });
    expect(callback).not.toHaveBeenCalled();
  });

  it("cancels a prior pending callback when a new one is scheduled", () => {
    const first = vi.fn();
    const second = vi.fn();
    const { result } = renderHook(() => useAutoCloseTimeout());

    act(() => {
      result.current.scheduleAutoClose(first, 1500);
      result.current.scheduleAutoClose(second, 1500);
    });

    act(() => {
      vi.advanceTimersByTime(1500);
    });
    expect(first).not.toHaveBeenCalled();
    expect(second).toHaveBeenCalledTimes(1);
  });
});
