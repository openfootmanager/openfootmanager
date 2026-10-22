import { readFileSync } from "node:fs";
import { describe, expect, it } from "vitest";

import { SIDEBAR_REM, sidebarLayout } from "./sidebarLayout";

const closed = { collapsed: false, overlayOpen: false } as const;

describe("sidebarLayout", () => {
  /**
   * Given the Normal behaviour, each width, each text size (root 14, 16, 18, 20px) and a 960 or
   * 1280 window
   * When the sidebar is laid out
   * Then it is open wherever the window holds the open sidebar plus a 40rem page, and a rail
   * otherwise: the same 24 cells the prototype's self-check measured.
   */
  it.each([
    [960, 14, "compact", "open"],
    [960, 14, "standard", "open"],
    [960, 14, "wide", "open"],
    [960, 16, "compact", "open"],
    [960, 16, "standard", "open"],
    [960, 16, "wide", "open"],
    [960, 18, "compact", "open"],
    [960, 18, "standard", "rail"],
    [960, 18, "wide", "rail"],
    [960, 20, "compact", "rail"],
    [960, 20, "standard", "rail"],
    [960, 20, "wide", "rail"],
    [1280, 14, "compact", "open"],
    [1280, 14, "standard", "open"],
    [1280, 14, "wide", "open"],
    [1280, 16, "compact", "open"],
    [1280, 16, "standard", "open"],
    [1280, 16, "wide", "open"],
    [1280, 18, "compact", "open"],
    [1280, 18, "standard", "open"],
    [1280, 18, "wide", "open"],
    [1280, 20, "compact", "open"],
    [1280, 20, "standard", "open"],
    [1280, 20, "wide", "open"],
  ] as const)("Normal at %ipx, %ipx text, %s: %s", (windowPx, rootPx, width, expected) => {
    const layout = sidebarLayout({ width, behaviour: "normal", ...closed }, { windowPx, rootPx });
    expect(layout.slot === "rail" ? "rail" : "open").toBe(expected);
    expect(layout.drawn).toBe(layout.slot);
    expect(layout.overlay).toBe(false);
  });

  /**
   * Given Normal with room for the sidebar
   * When the player has folded it, then unfolds it
   * Then it is a rail with an expand button, then open with a collapse button: the choice holds.
   */
  it("keeps the player's fold when there is room", () => {
    const frame = { windowPx: 1280, rootPx: 16 };
    const folded = sidebarLayout(
      { width: "standard", behaviour: "normal", collapsed: true, overlayOpen: false },
      frame,
    );
    expect(folded).toMatchObject({ slot: "rail", drawn: "rail", toggle: "expand" });
    const open = sidebarLayout({ width: "standard", behaviour: "normal", ...closed }, frame);
    expect(open).toMatchObject({ slot: "standard", drawn: "standard", toggle: "collapse" });
  });

  /**
   * Given Normal in a window too narrow for the open sidebar
   * When the player presses expand
   * Then the sidebar opens over the page with a backdrop, the page keeping the rail's width.
   */
  it("opens over the page as a drawer when the window is too narrow", () => {
    const frame = { windowPx: 960, rootPx: 20 };
    const rail = sidebarLayout({ width: "standard", behaviour: "normal", ...closed }, frame);
    expect(rail).toMatchObject({
      slot: "rail",
      drawn: "rail",
      toggle: "expand",
      opensDrawer: true,
    });
    const drawer = sidebarLayout(
      { width: "standard", behaviour: "normal", collapsed: false, overlayOpen: true },
      frame,
    );
    expect(drawer).toMatchObject({
      slot: "rail",
      drawn: "standard",
      overlay: true,
      backdrop: true,
      toggle: "collapse",
    });
  });

  /**
   * Given Expand on hover, Expand on click and Icons only, in a window with plenty of room
   * When they are laid out at rest, and opened
   * Then all three rest as a rail; hover opens over the page with no backdrop and no button; click
   * opens a drawer with a backdrop from its button; icons only never opens.
   */
  it("rests as a rail for hover, click and icons, and opens each its own way", () => {
    const frame = { windowPx: 1600, rootPx: 16 };
    const at = (behaviour: "hover" | "click" | "icons", overlayOpen: boolean) =>
      sidebarLayout({ width: "wide", behaviour, collapsed: false, overlayOpen }, frame);

    expect(at("hover", false)).toMatchObject({ slot: "rail", drawn: "rail", toggle: null });
    expect(at("hover", true)).toMatchObject({
      slot: "rail",
      drawn: "wide",
      overlay: true,
      backdrop: false,
    });
    expect(at("click", false)).toMatchObject({
      slot: "rail",
      drawn: "rail",
      toggle: "expand",
      opensDrawer: true,
    });
    expect(at("click", true)).toMatchObject({
      slot: "rail",
      drawn: "wide",
      overlay: true,
      backdrop: true,
      toggle: "collapse",
    });
    expect(at("icons", true)).toMatchObject({
      slot: "rail",
      drawn: "rail",
      overlay: false,
      toggle: null,
    });
  });

  /**
   * Given the widths the layout reckons with
   * When they are compared with the @theme tokens the sidebar is drawn with
   * Then they agree, so the fold threshold and the drawn width can't drift apart.
   */
  it("reckons with the same widths the sidebar is drawn at", () => {
    const appCss = readFileSync("src/App.css", "utf-8");
    const token = (name: string) =>
      Number(new RegExp(`--spacing-${name}:\\s*([\\d.]+)rem;`).exec(appCss)?.[1]);
    expect(token("sidebar-rail")).toBe(SIDEBAR_REM.rail);
    expect(token("sidebar-compact")).toBe(SIDEBAR_REM.compact);
    expect(token("sidebar-standard")).toBe(SIDEBAR_REM.standard);
    expect(token("sidebar-wide")).toBe(SIDEBAR_REM.wide);
  });
});
