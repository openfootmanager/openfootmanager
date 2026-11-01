import { describe, expect, it } from "vitest";

import {
  contrastRatio,
  DARK_INK,
  LIGHT_INK,
  oppositeInk,
  parseColor,
  readableInk,
  relativeLuminance,
} from "./colorContrast";

describe("parseColor", () => {
  /**
   * Given colours written the ways a club's colours arrive (six and three hex digits, rgb(), hsl(),
   *       a CSS name, with stray case and spaces)
   * When they are parsed
   * Then each reads as its sRGB value, and a value that is no colour reads as null.
   */
  it("reads the forms a club's colours come in", () => {
    expect(parseColor("#0a8f3a")).toEqual({ r: 10, g: 143, b: 58 });
    expect(parseColor(" #0F0 ")).toEqual({ r: 0, g: 255, b: 0 });
    expect(parseColor("#ff000080")).toEqual({ r: 255, g: 0, b: 0 });
    expect(parseColor("rgb(255, 128, 0)")).toEqual({ r: 255, g: 128, b: 0 });
    expect(parseColor("hsl(120, 100%, 25%)")).toEqual({ r: 0, g: 128, b: 0 });
    expect(parseColor("Green")).toEqual({ r: 0, g: 128, b: 0 });
    expect(parseColor("white")).toEqual({ r: 255, g: 255, b: 255 });
    expect(parseColor("not a colour")).toBeNull();
    expect(parseColor("#12")).toBeNull();
  });
});

describe("contrast", () => {
  /**
   * Given black and white, and two equal colours
   * When their contrast is measured
   * Then it is the WCAG extremes: 21 for black on white, 1 for a colour on itself.
   */
  it("measures contrast as WCAG does", () => {
    const black = { r: 0, g: 0, b: 0 };
    const white = { r: 255, g: 255, b: 255 };

    expect(relativeLuminance(white)).toBeCloseTo(1, 5);
    expect(contrastRatio(black, white)).toBeCloseTo(21, 5);
    expect(contrastRatio(white, black)).toBeCloseTo(21, 5);
    expect(contrastRatio(white, white)).toBeCloseTo(1, 5);
  });
});

describe("readableInk", () => {
  /**
   * Given a green and white crest (the club whose "AL" vanished on the white half)
   * When the label ink is chosen against both colours
   * Then it is the dark ink: white would have been invisible on the white half.
   */
  it("picks the ink that reads on both halves, not just the first colour", () => {
    expect(readableInk(["#00ff00", "#ffffff"])).toBe(DARK_INK);
    // The primary alone calls for white here; the white half is why it must not be.
    expect(readableInk(["#15803d", "#ffffff"])).toBe(DARK_INK);
    expect(readableInk(["#15803d"])).toBe(LIGHT_INK);
    expect(readableInk(["#003300", "#000000"])).toBe(LIGHT_INK);
    expect(readableInk(["#ffffff", "#ffff00"])).toBe(DARK_INK);
  });

  /**
   * Given colours in forms the old hex-only parser could not read (three digits, a name)
   * When the label ink is chosen
   * Then they count as the colours they are, not as white.
   */
  it("reads three-digit hex and named colours", () => {
    expect(readableInk(["#fff", "white"])).toBe(DARK_INK);
    expect(readableInk(["yellow", "#ff0"])).toBe(DARK_INK);
    expect(readableInk(["navy", "black"])).toBe(LIGHT_INK);
    // Read as three digits these are dark and call for white; unread they would be mid-grey.
    expect(readableInk(["#000", "#003"])).toBe(LIGHT_INK);
  });

  /**
   * Given a colour that is not a colour
   * When the ink is chosen
   * Then it counts as mid-grey beside the colour that is, instead of deciding the ink alone.
   */
  it("treats an unreadable colour as mid-grey", () => {
    expect(readableInk(["not a colour", "#ffffff"])).toBe(DARK_INK);
    expect(readableInk(["not a colour", "#000000"])).toBe(LIGHT_INK);
  });

  it("gives the halo the opposite ink", () => {
    expect(oppositeInk(DARK_INK)).toBe(LIGHT_INK);
    expect(oppositeInk(LIGHT_INK)).toBe(DARK_INK);
  });
});
