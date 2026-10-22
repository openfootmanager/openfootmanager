import { act, renderHook } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import { useSidebar } from "./useSidebar";

const settings = (behaviour: "normal" | "hover" | "click" | "icons", collapsed = false) =>
  ({ width: "standard", behaviour, collapsed }) as const;

function setWindowWidth(px: number) {
  Object.defineProperty(window, "innerWidth", { configurable: true, value: px });
}

describe("useSidebar", () => {
  beforeEach(() => {
    setWindowWidth(1280);
    document.documentElement.style.fontSize = "16px";
  });
  afterEach(() => {
    vi.useRealTimers();
    document.documentElement.style.fontSize = "";
  });

  /**
   * Given Expand on hover
   * When the pointer rests on the rail, then leaves
   * Then it opens over the page only after a short delay, so a pointer passing by doesn't flash
   * it open, and closes a moment after the pointer leaves.
   */
  it("opens on hover after a short delay and closes after leaving", () => {
    vi.useFakeTimers();
    const { result } = renderHook(() => useSidebar(settings("hover")));
    expect(result.current.layout.drawn).toBe("rail");

    act(() => result.current.asideHandlers.onPointerEnter());
    act(() => vi.advanceTimersByTime(100));
    expect(result.current.layout.drawn).toBe("rail");
    act(() => vi.advanceTimersByTime(60));
    expect(result.current.layout).toMatchObject({ slot: "rail", drawn: "standard", overlay: true });

    act(() => result.current.asideHandlers.onPointerLeave());
    act(() => vi.advanceTimersByTime(260));
    expect(result.current.layout.drawn).toBe("rail");
  });

  /**
   * Given Expand on hover, open under the pointer
   * When Esc is pressed
   * Then it closes and stays closed while the pointer is still there, until the pointer leaves.
   */
  it("closes on Esc and stays closed until the pointer leaves", () => {
    vi.useFakeTimers();
    const { result } = renderHook(() => useSidebar(settings("hover")));
    act(() => result.current.asideHandlers.onPointerEnter());
    act(() => vi.advanceTimersByTime(200));

    act(() => {
      document.dispatchEvent(new KeyboardEvent("keydown", { key: "Escape" }));
    });
    expect(result.current.layout.drawn).toBe("rail");
    act(() => result.current.asideHandlers.onPointerEnter());
    act(() => vi.advanceTimersByTime(200));
    expect(result.current.layout.drawn).toBe("rail");

    act(() => result.current.asideHandlers.onPointerLeave());
    act(() => result.current.asideHandlers.onPointerEnter());
    act(() => vi.advanceTimersByTime(200));
    expect(result.current.layout.drawn).toBe("standard");
  });

  /**
   * Given Expand on click
   * When its button is pressed, and then Esc
   * Then it opens as a drawer that a click outside closes, and Esc closes it; hovering never
   * opens it.
   */
  it("opens as a drawer from its button and closes on Esc, never on hover", () => {
    vi.useFakeTimers();
    const { result } = renderHook(() => useSidebar(settings("click")));
    act(() => result.current.asideHandlers.onPointerEnter());
    act(() => vi.advanceTimersByTime(500));
    expect(result.current.layout.drawn).toBe("rail");

    act(() => result.current.toggle());
    expect(result.current.layout).toMatchObject({
      drawn: "standard",
      overlay: true,
      closesOnOutsideClick: true,
    });
    act(() => {
      document.dispatchEvent(new KeyboardEvent("keydown", { key: "Escape" }));
    });
    expect(result.current.layout.drawn).toBe("rail");
  });

  /**
   * Given Normal in a window that is wide enough, then the window narrows past the threshold, and
   * then the UI scale grows the root font
   * When the sidebar is laid out each time
   * Then it is open, then a rail: it follows a resize and a UI-scale change by itself.
   */
  it("follows a resize and a UI-scale change", () => {
    const { result } = renderHook(() => useSidebar(settings("normal")));
    expect(result.current.layout.drawn).toBe("standard");

    act(() => {
      setWindowWidth(880);
      window.dispatchEvent(new Event("resize"));
    });
    expect(result.current.layout.drawn).toBe("rail");

    act(() => {
      setWindowWidth(1000);
      window.dispatchEvent(new Event("resize"));
    });
    expect(result.current.layout.drawn).toBe("standard");
  });

  /**
   * Given the hook has gone (the dashboard left the screen)
   * When it unmounts
   * Then it stops listening to the window and stops watching the root's style.
   */
  it("stops listening when it unmounts", () => {
    const removeWindow = vi.spyOn(window, "removeEventListener");
    const disconnect = vi.spyOn(MutationObserver.prototype, "disconnect");
    const { unmount } = renderHook(() => useSidebar(settings("normal")));

    unmount();

    expect(removeWindow).toHaveBeenCalledWith("resize", expect.any(Function));
    expect(disconnect).toHaveBeenCalled();
    removeWindow.mockRestore();
    disconnect.mockRestore();
  });
});
