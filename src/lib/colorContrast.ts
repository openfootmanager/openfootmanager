/** An sRGB colour, each channel 0 to 255. */
export interface Rgb {
  r: number;
  g: number;
  b: number;
}

/** The CSS named colours (CSS Color 4), as `rrggbb`. */
const NAMED_COLORS: Record<string, string> = {
  aliceblue: "f0f8ff",
  antiquewhite: "faebd7",
  aqua: "00ffff",
  aquamarine: "7fffd4",
  azure: "f0ffff",
  beige: "f5f5dc",
  bisque: "ffe4c4",
  black: "000000",
  blanchedalmond: "ffebcd",
  blue: "0000ff",
  blueviolet: "8a2be2",
  brown: "a52a2a",
  burlywood: "deb887",
  cadetblue: "5f9ea0",
  chartreuse: "7fff00",
  chocolate: "d2691e",
  coral: "ff7f50",
  cornflowerblue: "6495ed",
  cornsilk: "fff8dc",
  crimson: "dc143c",
  cyan: "00ffff",
  darkblue: "00008b",
  darkcyan: "008b8b",
  darkgoldenrod: "b8860b",
  darkgray: "a9a9a9",
  darkgreen: "006400",
  darkgrey: "a9a9a9",
  darkkhaki: "bdb76b",
  darkmagenta: "8b008b",
  darkolivegreen: "556b2f",
  darkorange: "ff8c00",
  darkorchid: "9932cc",
  darkred: "8b0000",
  darksalmon: "e9967a",
  darkseagreen: "8fbc8f",
  darkslateblue: "483d8b",
  darkslategray: "2f4f4f",
  darkslategrey: "2f4f4f",
  darkturquoise: "00ced1",
  darkviolet: "9400d3",
  deeppink: "ff1493",
  deepskyblue: "00bfff",
  dimgray: "696969",
  dimgrey: "696969",
  dodgerblue: "1e90ff",
  firebrick: "b22222",
  floralwhite: "fffaf0",
  forestgreen: "228b22",
  fuchsia: "ff00ff",
  gainsboro: "dcdcdc",
  ghostwhite: "f8f8ff",
  gold: "ffd700",
  goldenrod: "daa520",
  gray: "808080",
  green: "008000",
  greenyellow: "adff2f",
  grey: "808080",
  honeydew: "f0fff0",
  hotpink: "ff69b4",
  indianred: "cd5c5c",
  indigo: "4b0082",
  ivory: "fffff0",
  khaki: "f0e68c",
  lavender: "e6e6fa",
  lavenderblush: "fff0f5",
  lawngreen: "7cfc00",
  lemonchiffon: "fffacd",
  lightblue: "add8e6",
  lightcoral: "f08080",
  lightcyan: "e0ffff",
  lightgoldenrodyellow: "fafad2",
  lightgray: "d3d3d3",
  lightgreen: "90ee90",
  lightgrey: "d3d3d3",
  lightpink: "ffb6c1",
  lightsalmon: "ffa07a",
  lightseagreen: "20b2aa",
  lightskyblue: "87cefa",
  lightslategray: "778899",
  lightslategrey: "778899",
  lightsteelblue: "b0c4de",
  lightyellow: "ffffe0",
  lime: "00ff00",
  limegreen: "32cd32",
  linen: "faf0e6",
  magenta: "ff00ff",
  maroon: "800000",
  mediumaquamarine: "66cdaa",
  mediumblue: "0000cd",
  mediumorchid: "ba55d3",
  mediumpurple: "9370db",
  mediumseagreen: "3cb371",
  mediumslateblue: "7b68ee",
  mediumspringgreen: "00fa9a",
  mediumturquoise: "48d1cc",
  mediumvioletred: "c71585",
  midnightblue: "191970",
  mintcream: "f5fffa",
  mistyrose: "ffe4e1",
  moccasin: "ffe4b5",
  navajowhite: "ffdead",
  navy: "000080",
  oldlace: "fdf5e6",
  olive: "808000",
  olivedrab: "6b8e23",
  orange: "ffa500",
  orangered: "ff4500",
  orchid: "da70d6",
  palegoldenrod: "eee8aa",
  palegreen: "98fb98",
  paleturquoise: "afeeee",
  palevioletred: "db7093",
  papayawhip: "ffefd5",
  peachpuff: "ffdab9",
  peru: "cd853f",
  pink: "ffc0cb",
  plum: "dda0dd",
  powderblue: "b0e0e6",
  purple: "800080",
  rebeccapurple: "663399",
  red: "ff0000",
  rosybrown: "bc8f8f",
  royalblue: "4169e1",
  saddlebrown: "8b4513",
  salmon: "fa8072",
  sandybrown: "f4a460",
  seagreen: "2e8b57",
  seashell: "fff5ee",
  sienna: "a0522d",
  silver: "c0c0c0",
  skyblue: "87ceeb",
  slateblue: "6a5acd",
  slategray: "708090",
  slategrey: "708090",
  snow: "fffafa",
  springgreen: "00ff7f",
  steelblue: "4682b4",
  tan: "d2b48c",
  teal: "008080",
  thistle: "d8bfd8",
  tomato: "ff6347",
  turquoise: "40e0d0",
  violet: "ee82ee",
  wheat: "f5deb3",
  white: "ffffff",
  whitesmoke: "f5f5f5",
  yellow: "ffff00",
  yellowgreen: "9acd32",
};

function fromHex(hex: string): Rgb | null {
  const digits = hex.length === 3 || hex.length === 4 ? [...hex].map((c) => c + c).join("") : hex;
  if (digits.length !== 6 && digits.length !== 8) return null;
  const value = Number.parseInt(digits.slice(0, 6), 16);
  return { r: (value >> 16) & 0xff, g: (value >> 8) & 0xff, b: value & 0xff };
}

function fromFunction(name: string, args: string): Rgb | null {
  const parts = args
    .split(/[\s,/]+/)
    .filter(Boolean)
    .map((part) => Number.parseFloat(part));
  if (parts.length < 3 || parts.slice(0, 3).some(Number.isNaN)) return null;
  const [a, b, c] = parts;
  if (name === "rgb" || name === "rgba") {
    const percent = args.includes("%");
    const channel = (value: number) =>
      Math.min(255, Math.max(0, Math.round(percent ? (value / 100) * 255 : value)));
    return { r: channel(a), g: channel(b), b: channel(c) };
  }
  // hsl(h, s%, l%): the crest derives its own colours in this form.
  const hue = (((a % 360) + 360) % 360) / 360;
  const s = Math.min(1, Math.max(0, b / 100));
  const l = Math.min(1, Math.max(0, c / 100));
  const q = l < 0.5 ? l * (1 + s) : l + s - l * s;
  const p = 2 * l - q;
  const part = (offset: number) => {
    let t = hue + offset;
    if (t < 0) t += 1;
    if (t > 1) t -= 1;
    if (t < 1 / 6) return p + (q - p) * 6 * t;
    if (t < 1 / 2) return q;
    if (t < 2 / 3) return p + (q - p) * (2 / 3 - t) * 6;
    return p;
  };
  return {
    r: Math.round(part(1 / 3) * 255),
    g: Math.round(part(0) * 255),
    b: Math.round(part(-1 / 3) * 255),
  };
}

/** Reads a CSS colour: #rgb, #rrggbb (alpha ignored), rgb(), hsl() or a named colour; else null. */
export function parseColor(value: string): Rgb | null {
  const text = value.trim().toLowerCase();
  if (text.startsWith("#")) return fromHex(text.slice(1));
  const call = /^(rgba?|hsla?)\(([^)]*)\)$/.exec(text);
  if (call) return fromFunction(call[1], call[2]);
  const named = NAMED_COLORS[text];
  return named ? fromHex(named) : null;
}

/** WCAG relative luminance, 0 (black) to 1 (white). */
export function relativeLuminance({ r, g, b }: Rgb): number {
  const linear = (channel: number) => {
    const c = channel / 255;
    return c <= 0.04045 ? c / 12.92 : ((c + 0.055) / 1.055) ** 2.4;
  };
  return 0.2126 * linear(r) + 0.7152 * linear(g) + 0.0722 * linear(b);
}

/** WCAG contrast ratio of two colours, 1 (none) to 21 (black on white). */
export function contrastRatio(first: Rgb, second: Rgb): number {
  const [light, dark] = [relativeLuminance(first), relativeLuminance(second)].sort((a, b) => b - a);
  return (light + 0.05) / (dark + 0.05);
}

/** Near-black and white, the two inks a label is drawn in over club colours. */
export const DARK_INK = "#111827";
export const LIGHT_INK = "#ffffff";

/** A colour that cannot be read counts as mid-grey: neither ink is then assumed to suit it. */
const MID_GREY: Rgb = { r: 128, g: 128, b: 128 };

/**
 * The ink (near-black or white) whose worst contrast against any of the `backgrounds` is the best
 * of the two, so a label over a tile of two colours reads on both halves. Ties go to white.
 */
export function readableInk(backgrounds: readonly string[]): string {
  const colors = backgrounds.map((value) => parseColor(value) ?? MID_GREY);
  const dark = parseColor(DARK_INK) as Rgb;
  const light = parseColor(LIGHT_INK) as Rgb;
  const worst = (ink: Rgb) => Math.min(...colors.map((color) => contrastRatio(ink, color)));
  return worst(dark) > worst(light) ? DARK_INK : LIGHT_INK;
}

/** The ink opposite `ink`, for the halo that keeps a label legible where the colours disagree. */
export function oppositeInk(ink: string): string {
  return ink === DARK_INK ? LIGHT_INK : DARK_INK;
}
