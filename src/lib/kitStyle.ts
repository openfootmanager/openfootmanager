import type { CSSProperties } from "react";

import type { TeamColors } from "../store/types";

const HEX_COLOUR = /^#[0-9a-f]{6}$/i;

/**
 * A club's kit colours as the --kit and --kit-2 variables that App.css's kit tokens read (the
 * card's stripe, its washed header and the hairline). The wash itself is mixed in one place, the
 * `kit-wash` token, so a card never works a colour out. Colours are data: anything but a
 * six-digit hex is dropped, and the card falls back to its neutral look.
 */
export function kitStyle(kit: TeamColors | null | undefined): CSSProperties | undefined {
  if (!kit || !HEX_COLOUR.test(kit.primary) || !HEX_COLOUR.test(kit.secondary)) return undefined;
  return { "--kit": kit.primary, "--kit-2": kit.secondary } as CSSProperties;
}
