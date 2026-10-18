import { rootFontSizeFor, type UiScale } from "../../lib/uiScale";

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
