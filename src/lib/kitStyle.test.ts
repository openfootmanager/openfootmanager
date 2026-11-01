import { describe, expect, it } from "vitest";

import { kitStyle, kitWashScale } from "./kitStyle";

describe("kitStyle", () => {
  /**
   * Given a club's kit colours from the backend
   * When a card asks for its style
   * Then it gets the two colours as the --kit and --kit-2 variables, which App.css's kit tokens
   * read, so no component holds a colour of its own.
   */
  it("hands a club's kit colours to the kit tokens", () => {
    expect(kitStyle({ primary: "#1e3a8a", secondary: "#FFFFFF" })).toEqual({
      "--kit": "#1e3a8a",
      "--kit-2": "#FFFFFF",
      "--kit-wash-scale": "1",
    });
  });

  /**
   * Given a pale kit (white, yellow) and a dark one (navy)
   * When a card asks for its style
   * Then the pale kit's header wash takes a share scaled down by its luminance (white a quarter),
   * so it can't lift a dark header past readable, and the dark kit takes the full share.
   */
  it("scales a pale kit's header wash down, and leaves a dark kit's whole", () => {
    expect(kitWashScale("#ffffff")).toBeCloseTo(0.25, 3);
    expect(kitWashScale("#ffff00")).toBeLessThan(0.3);
    expect(kitWashScale("#1e3a8a")).toBe(1);
    expect(kitWashScale("#000000")).toBe(1);
    expect(kitStyle({ primary: "#ffffff", secondary: "#000000" })).toMatchObject({
      "--kit-wash-scale": "0.25",
    });
  });

  /**
   * Given no kit (a free agent, an unemployed manager) or colours that aren't six-digit hex
   * When a card asks for its style
   * Then it gets none, so the card falls back to its neutral stripe and wash instead of writing
   * whatever the data held into a style.
   */
  it("gives no style without a kit, or for colours that aren't hex", () => {
    expect(kitStyle(null)).toBeUndefined();
    expect(kitStyle(undefined)).toBeUndefined();
    expect(kitStyle({ primary: "red", secondary: "#ffffff" })).toBeUndefined();
    expect(kitStyle({ primary: "#16a34a", secondary: "url(x)" })).toBeUndefined();
    expect(kitStyle({ primary: "#fff", secondary: "#000" })).toBeUndefined();
  });
});
