import type { CSSProperties } from "react";

import type { TeamColors } from "../store/types";
import { parseColor, relativeLuminance } from "./colorContrast";

const HEX_COLOUR = /^#[0-9a-f]{6}$/i;

/**
 * Kits no brighter than this wash a card header at the full --kit-wash-mix. A paler kit washes it
 * at a share scaled down by its luminance: on a dark panel, a white, yellow or lime kit at the full
 * share would lift the header so far that its meta line (fg-muted) dropped under 4.5:1.
 */
const FULL_WASH_LUMINANCE = 0.25;

/** How much of the --kit-wash-mix share a kit's header wash takes: 1, or less for a pale kit. */
export function kitWashScale(primary: string): number {
  const colour = parseColor(primary);
  if (!colour) return 1;
  const luminance = relativeLuminance(colour);
  return luminance <= FULL_WASH_LUMINANCE ? 1 : FULL_WASH_LUMINANCE / luminance;
}

/**
 * A club's kit colours as the --kit and --kit-2 variables that App.css's kit tokens read (the
 * card's stripe, its washed header and the hairline), with --kit-wash-scale for a pale kit. The
 * wash itself is mixed in one place, the `kit-wash` token, so a card never works a colour out.
 * Colours are data: anything but a six-digit hex is dropped, and the card falls back to its
 * neutral look.
 */
export function kitStyle(kit: TeamColors | null | undefined): CSSProperties | undefined {
  if (!kit || !HEX_COLOUR.test(kit.primary) || !HEX_COLOUR.test(kit.secondary)) return undefined;
  return {
    "--kit": kit.primary,
    "--kit-2": kit.secondary,
    "--kit-wash-scale": String(Math.round(kitWashScale(kit.primary) * 1000) / 1000),
  } as CSSProperties;
}
