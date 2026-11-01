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

describe("parseColor, the edges", () => {
  /**
   * Given a hex value whose digits are not hexadecimal
   * When it is parsed
   * Then it is no colour, instead of black.
   */
  it("rejects non-hexadecimal digits", () => {
    expect(parseColor("#gggggg")).toBeNull();
    expect(parseColor("#12x")).toBeNull();
  });

  /**
   * Given an opaque rgb() written with a percentage alpha
   * When it is parsed
   * Then the channels stay numbers: the percent belongs to the alpha, not to them.
   */
  it("reads the slash value of rgb() as alpha, not as a unit of the channels", () => {
    expect(parseColor("rgb(50 50 50 / 100%)")).toEqual({ r: 50, g: 50, b: 50 });
    // No spaces round the slash: the alpha must still not turn the channels into percentages.
    expect(parseColor("rgb(50 50 50/100%)")).toEqual({ r: 50, g: 50, b: 50 });
    expect(parseColor("rgb(100% 0% 0%)")).toEqual({ r: 255, g: 0, b: 0 });
  });

  /**
   * Given hsl() hues in other angle units
   * When they are parsed
   * Then they mean the same colour as the hue in degrees.
   */
  it("converts the hue's unit to degrees", () => {
    const cyan = parseColor("hsl(180, 100%, 50%)");

    expect(cyan).toEqual({ r: 0, g: 255, b: 255 });
    expect(parseColor("hsl(0.5turn 100% 50%)")).toEqual(cyan);
    expect(parseColor("hsl(200grad 100% 50%)")).toEqual(cyan);
    expect(parseColor("hsl(3.14159rad 100% 50%)")).toEqual(cyan);
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
    // A mid-tone, from the WCAG reference: #777777 on white is 4.48:1 (the well-known "just short of
    // 4.5" grey), so a slip in the curve between the extremes shows.
    expect(contrastRatio({ r: 0x77, g: 0x77, b: 0x77 }, white)).toBeCloseTo(4.48, 2);
    // A dark grey whose channel sits just above the curve's linear segment (#141414, 20/255): 18.42,
    // where reading it on the linear segment would say 18.73.
    expect(contrastRatio({ r: 20, g: 20, b: 20 }, white)).toBeCloseTo(18.42, 2);
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
