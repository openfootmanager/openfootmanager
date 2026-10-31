import { describe, expect, it } from "vitest";

import { kitStyle } from "./kitStyle";

describe("kitStyle", () => {
  /**
   * Given a club's kit colours from the backend
   * When a card asks for its style
   * Then it gets the two colours as the --kit and --kit-2 variables, which App.css's kit tokens
   * read, so no component holds a colour of its own.
   */
  it("hands a club's kit colours to the kit tokens", () => {
    expect(kitStyle({ primary: "#16a34a", secondary: "#FFFFFF" })).toEqual({
      "--kit": "#16a34a",
      "--kit-2": "#FFFFFF",
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
