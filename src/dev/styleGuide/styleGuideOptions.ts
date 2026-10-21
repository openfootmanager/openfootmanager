import { rootFontSizeFor, type UiScale } from "../../lib/uiScale";

/**
 * The guide keeps its theme under its own key: it runs on the same dev origin as the app, and
 * writing the app's key would change the theme the app opens in.
 */
export const STYLE_GUIDE_THEME_KEY = "ofm-style-guide-theme";

export interface StyleGuideOptions {
  theme: "light" | "dark";
  density: "comfortable" | "compact";
  scale: UiScale;
  highContrast: boolean;
  rootFontSize: string;
  dialogOpen: boolean;
}

function pick<T extends string>(value: string | null, allowed: readonly T[], fallback: T): T {
  return allowed.find((option) => option === value) ?? fallback;
}

/**
 * Puts the options on the document the way the app would: the theme, density and high-contrast
 * classes and the ui_scale root size. ThemeProvider reads its starting theme from storage, so the
 * guide's own toggle starts from the URL's theme too.
 */
export function applyStyleGuideOptions(
  options: StyleGuideOptions,
  root: HTMLElement,
  storage: Pick<Storage, "setItem">,
): void {
  root.classList.toggle("dark", options.theme === "dark");
  root.classList.toggle("density-compact", options.density === "compact");
  root.classList.toggle("high-contrast", options.highContrast);
  root.style.fontSize = options.rootFontSize;
  storage.setItem(STYLE_GUIDE_THEME_KEY, options.theme);
}

/** The guide's settings, from the URL so a screenshot can ask for any combination. */
export function readStyleGuideOptions(search: string): StyleGuideOptions {
  const params = new URLSearchParams(search);
  const scale = pick<UiScale>(
    params.get("scale"),
    ["small", "normal", "large", "xlarge"],
    "normal",
  );
  return {
    theme: pick(params.get("theme"), ["light", "dark"] as const, "light"),
    density: pick(params.get("density"), ["comfortable", "compact"] as const, "comfortable"),
    scale,
    highContrast: params.get("contrast") === "high",
    rootFontSize: rootFontSizeFor(scale),
    dialogOpen: params.get("dialog") === "open",
  };
}
