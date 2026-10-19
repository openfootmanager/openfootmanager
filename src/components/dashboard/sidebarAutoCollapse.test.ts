import { act, renderHook } from "@testing-library/react";
import { afterEach, describe, expect, it } from "vitest";

import { sidebarMustCollapse, useSidebarMustCollapse } from "./sidebarAutoCollapse";

describe("sidebarMustCollapse", () => {
  /**
   * Given the 960px minimum window at normal scale (16px root),
   * then the workspace beside an expanded sidebar is 44rem, over the 40rem floor, so it stays open.
   */
  it("keeps the sidebar open at 960 wide and normal scale", () => {
    expect(sidebarMustCollapse(960, 16)).toBe(false);
  });

  /**
   * Given the same window at large (18px) or extra large (20px) scale,
   * then the workspace would fall under 40rem, so the sidebar collapses to its icon rail.
   */
  it("collapses it at 960 wide and large or extra-large scale", () => {
    expect(sidebarMustCollapse(960, 18)).toBe(true);
    expect(sidebarMustCollapse(960, 20)).toBe(true);
  });

  /** Given a 1280px window at extra-large scale, then there is room: it stays open. */
  it("keeps it open at 1280 wide, even at extra-large scale", () => {
    expect(sidebarMustCollapse(1280, 20)).toBe(false);
  });
});

describe("useSidebarMustCollapse", () => {
  afterEach(() => {
    document.documentElement.style.fontSize = "";
  });

  /**
   * Given a 960px window,
   * when the UI scale changes the root size from 20px to 16px,
   * then the answer follows it without a window resize, as changing the setting would.
   */
  it("follows a UI-scale change", async () => {
    Object.defineProperty(window, "innerWidth", { configurable: true, value: 960 });
    document.documentElement.style.fontSize = "20px";
    const { result } = renderHook(() => useSidebarMustCollapse());
    expect(result.current).toBe(true);

    await act(async () => {
      document.documentElement.style.fontSize = "16px";
      await Promise.resolve();
    });
    expect(result.current).toBe(false);
  });

  /**
   * Given an 880px window at normal scale (under 56rem = 896px, so folded),
   * when the window grows to 1280,
   * then the sidebar opens again.
   */
  it("follows a window resize", () => {
    Object.defineProperty(window, "innerWidth", { configurable: true, value: 880 });
    document.documentElement.style.fontSize = "16px";
    const { result } = renderHook(() => useSidebarMustCollapse());
    expect(result.current).toBe(true);

    act(() => {
      Object.defineProperty(window, "innerWidth", { configurable: true, value: 1280 });
      window.dispatchEvent(new Event("resize"));
    });
    expect(result.current).toBe(false);
  });
});
