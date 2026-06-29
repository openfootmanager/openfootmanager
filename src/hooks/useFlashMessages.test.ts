import { act, renderHook } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { useFlashMessages } from "./useFlashMessages";

describe("useFlashMessages", () => {
  beforeEach(() => {
    vi.useFakeTimers();
  });

  afterEach(() => {
    vi.useRealTimers();
  });

  it("starts with no messages", () => {
    const { result } = renderHook(() => useFlashMessages());
    expect(result.current.errorMsg).toBeNull();
    expect(result.current.successMsg).toBeNull();
  });

  it("flashError sets the error message and clears it after 5s", () => {
    const { result } = renderHook(() => useFlashMessages());

    act(() => {
      result.current.flashError("boom");
    });
    expect(result.current.errorMsg).toBe("boom");
    expect(result.current.successMsg).toBeNull();

    act(() => {
      vi.advanceTimersByTime(5000);
    });
    expect(result.current.errorMsg).toBeNull();
  });

  it("flashSuccess sets the success message and clears it after 5s", () => {
    const { result } = renderHook(() => useFlashMessages());

    act(() => {
      result.current.flashSuccess("done");
    });
    expect(result.current.successMsg).toBe("done");
    expect(result.current.errorMsg).toBeNull();

    act(() => {
      vi.advanceTimersByTime(5000);
    });
    expect(result.current.successMsg).toBeNull();
  });

  it("error and success messages are independent", () => {
    const { result } = renderHook(() => useFlashMessages());

    act(() => {
      result.current.flashError("err");
      result.current.flashSuccess("ok");
    });
    expect(result.current.errorMsg).toBe("err");
    expect(result.current.successMsg).toBe("ok");
  });

  it("a rapid second flash resets its own timer instead of being cleared early", () => {
    const { result } = renderHook(() => useFlashMessages());

    act(() => {
      result.current.flashError("first");
    });
    // Almost-but-not-quite expire, then flash again.
    act(() => {
      vi.advanceTimersByTime(4000);
      result.current.flashError("second");
    });
    expect(result.current.errorMsg).toBe("second");

    // The first timer (had it survived) would fire here; it must not clear.
    act(() => {
      vi.advanceTimersByTime(1000);
    });
    expect(result.current.errorMsg).toBe("second");

    // The second timer's full duration clears it.
    act(() => {
      vi.advanceTimersByTime(4000);
    });
    expect(result.current.errorMsg).toBeNull();
  });

  it("clears the pending timer on unmount", () => {
    const { result, unmount } = renderHook(() => useFlashMessages());

    act(() => {
      result.current.flashError("boom");
    });
    expect(vi.getTimerCount()).toBe(1);

    unmount();
    // Without the cleanup effect this timer would survive and fire setState
    // after unmount.
    expect(vi.getTimerCount()).toBe(0);
  });
});
