import { act, renderHook } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import { useWidthAtLeast } from "./useWidthAtLeast";

describe("useWidthAtLeast", () => {
  afterEach(() => {
    vi.unstubAllGlobals();
  });

  /**
   * Given an element 1000px wide at a 16px root, observed for resizes
   * When it is asked whether it is 72rem (1152px) wide, and then it grows to 1200px
   * Then it is not, and then it is: the answer follows the element.
   */
  it("answers for the element's width as it resizes", () => {
    let report = () => {};
    vi.stubGlobal(
      "ResizeObserver",
      class {
        constructor(callback: () => void) {
          report = callback;
        }
        observe(): void {}
        disconnect(): void {}
      },
    );
    const element = document.createElement("div");
    let width = 1000;
    element.getBoundingClientRect = () => ({ width }) as DOMRect;

    const { result } = renderHook(() => useWidthAtLeast({ current: element }, 72));
    expect(result.current).toBe(false);

    width = 1200;
    act(() => report());
    expect(result.current).toBe(true);
  });

  /**
   * Given a browser that can't observe sizes
   * When the hook runs
   * Then it answers yes, so the screen lays out as it does when there is room.
   */
  it("answers yes where nothing can be measured", () => {
    vi.stubGlobal("ResizeObserver", undefined);

    const { result } = renderHook(() =>
      useWidthAtLeast({ current: document.createElement("div") }, 72),
    );

    expect(result.current).toBe(true);
  });
});
