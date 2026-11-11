import { act, renderHook, waitFor } from "@testing-library/react";
import { afterEach, describe, expect, it } from "vitest";

import { useRootFontPx } from "./useRootFontPx";

afterEach(() => {
  document.documentElement.style.fontSize = "";
});

describe("useRootFontPx", () => {
  /**
   * Given a root font size of 20px (a larger UI scale)
   * When the hook is read
   * Then it reports 20, not a fixed 16
   */
  it("reports the root font size", () => {
    document.documentElement.style.fontSize = "20px";

    const { result } = renderHook(() => useRootFontPx());

    expect(result.current).toBe(20);
  });

  /**
   * Given the hook mounted at the default size
   * When the UI scale changes the root font size
   * Then the hook follows it
   */
  it("follows the UI scale when it changes", async () => {
    document.documentElement.style.fontSize = "16px";
    const { result } = renderHook(() => useRootFontPx());

    act(() => {
      document.documentElement.style.fontSize = "24px";
    });

    await waitFor(() => expect(result.current).toBe(24));
  });
});
