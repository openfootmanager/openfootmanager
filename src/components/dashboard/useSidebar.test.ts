import { act, renderHook } from "@testing-library/react";
import type { FocusEvent } from "react";
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
   * Given Expand on hover and a keyboard player tabbing into the rail
   * When the focus enters, moves between its items, then leaves the sidebar
   * Then it opens at once (no hover delay for the keyboard), stays open while the focus is inside,
   * and closes when the focus leaves.
   */
  it("opens on keyboard focus and closes when the focus leaves the sidebar", () => {
    vi.useFakeTimers();
    const { result } = renderHook(() => useSidebar(settings("hover")));
    const aside = document.createElement("aside");
    const item = aside.appendChild(document.createElement("button"));
    const blurTo = (next: Element | null) =>
      ({ currentTarget: aside, relatedTarget: next }) as unknown as FocusEvent<HTMLElement>;

    act(() => result.current.asideHandlers.onFocus());
    expect(result.current.layout).toMatchObject({ drawn: "standard", overlay: true });

    act(() => result.current.asideHandlers.onBlur(blurTo(item)));
    expect(result.current.layout.drawn).toBe("standard");

    act(() => result.current.asideHandlers.onBlur(blurTo(document.body)));
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
   * Given Normal in a narrow window, the drawer opened from its button
   * When the window widens until the sidebar sits open in the page, Esc is pressed (say to close a
   * dialog), and the window narrows again
   * Then the drawer is gone with the overlay: Esc is not the sidebar's (its focus stays put), and
   * the narrow window shows the rail, not a drawer left open.
   */
  it("forgets an open drawer once the window is wide enough to hold the sidebar", () => {
    setWindowWidth(880);
    const onEscape = vi.fn();
    const { result } = renderHook(() => useSidebar(settings("normal"), onEscape));
    act(() => result.current.toggle());
    expect(result.current.layout).toMatchObject({ slot: "rail", overlay: true });

    act(() => {
      setWindowWidth(1280);
      window.dispatchEvent(new Event("resize"));
    });
    expect(result.current.layout).toMatchObject({ slot: "standard", overlay: false });

    act(() => {
      document.dispatchEvent(new KeyboardEvent("keydown", { key: "Escape" }));
    });
    expect(onEscape).not.toHaveBeenCalled();

    act(() => {
      setWindowWidth(880);
      window.dispatchEvent(new Event("resize"));
    });
    expect(result.current.layout).toMatchObject({ drawn: "rail", overlay: false });
  });

  /**
   * Given Expand on click, the drawer open over the page with the focus on one of its items
   * When the focus moves to another of its items, is lost to nothing (a click on the drawer's own
   * padding), and then moves on to the page (Tab to the header's search)
   * Then the drawer stays open while the focus is inside or nowhere, and closes once the focus is
   * on the page: a drawer never sits over the focused element (WCAG 2.4.11).
   */
  it.each(["click", "normal"] as const)(
    "closes the %s drawer when the focus moves on to the page",
    (behaviour) => {
      if (behaviour === "normal") setWindowWidth(880);
      const { result } = renderHook(() => useSidebar(settings(behaviour)));
      act(() => result.current.toggle());
      expect(result.current.layout.overlay).toBe(true);
      const aside = document.createElement("aside");
      const item = aside.appendChild(document.createElement("button"));
      const search = document.createElement("input");
      const blurTo = (next: Element | null) =>
        ({ currentTarget: aside, relatedTarget: next }) as unknown as FocusEvent<HTMLElement>;

      act(() => result.current.asideHandlers.onBlur(blurTo(item)));
      act(() => result.current.asideHandlers.onBlur(blurTo(null)));
      expect(result.current.layout.overlay).toBe(true);

      act(() => result.current.asideHandlers.onBlur(blurTo(search)));
      expect(result.current.layout.overlay).toBe(false);
    },
  );

  /**
   * Given the drawer open, and a dialog open over the page (Exit's confirmation)
   * When Esc is pressed inside the dialog
   * Then the Esc is the dialog's: the drawer stays as it is and its button doesn't take the focus.
   */
  it("leaves an Esc pressed in a dialog to the dialog", () => {
    const onEscape = vi.fn();
    const { result } = renderHook(() => useSidebar(settings("click"), onEscape));
    act(() => result.current.toggle());
    const dialog = document.body.appendChild(document.createElement("div"));
    dialog.setAttribute("role", "dialog");
    const confirm = dialog.appendChild(document.createElement("button"));

    act(() => {
      confirm.dispatchEvent(new KeyboardEvent("keydown", { key: "Escape", bubbles: true }));
    });
    expect(onEscape).not.toHaveBeenCalled();
    expect(result.current.layout.overlay).toBe(true);
    dialog.remove();
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
