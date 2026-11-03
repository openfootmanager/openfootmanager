import { describe, expect, it } from "vitest";

import { contextMenuPoint } from "./contextMenuPoint";

/** An anchor whose box sits at left 120, bottom 340. */
function anchorAt(left: number, bottom: number): Element {
  const anchor = document.createElement("div");
  anchor.getBoundingClientRect = () =>
    ({ left, bottom, top: bottom - 40, right: left + 200, width: 200, height: 40 }) as DOMRect;
  return anchor;
}

describe("contextMenuPoint", () => {
  /**
   * Given a right-click with the mouse, at 300,200
   * When the menu's place is worked out
   * Then it opens at the pointer.
   */
  it("opens a mouse right-click's menu at the pointer", () => {
    expect(contextMenuPoint({ clientX: 300, clientY: 200 }, anchorAt(120, 340))).toEqual({
      x: 300,
      y: 200,
    });
  });

  /**
   * Given the Menu key or Shift+F10, which report their right-click at 0,0
   * When the menu's place is worked out
   * Then it opens under the anchor's left corner, not in the viewport's top corner.
   */
  it("opens a keyboard right-click's menu under its anchor", () => {
    expect(contextMenuPoint({ clientX: 0, clientY: 0 }, anchorAt(120, 340))).toEqual({
      x: 120,
      y: 340,
    });
  });
});
