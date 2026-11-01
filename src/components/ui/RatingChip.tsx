import { useTranslation } from "react-i18next";

import { ratingTierClass, ratingTierLetterKey } from "../../lib/ratingTier";
import type { RatingTier } from "../../services/squadService";

interface RatingChipProps {
  /** The overall rating; shown as the whole number the backend's tiers are keyed by. */
  value: number;
  /** The backend's tier for it (`useRatingTiers`): the chip paints it, and never picks it. */
  tier: RatingTier;
  /** `sm` fits a table row (the default); `md` a card or a profile line. */
  size?: "sm" | "md";
  /**
   * The translated name of what is rated, when it is not the overall rating ("Shooting"): the
   * chip then reads "Shooting 74, tier A" rather than calling it the overall.
   */
  name?: string;
}

const SIZE = {
  sm: "h-5 min-w-7 px-1.5 text-caption",
  md: "h-6 min-w-8 px-2 text-body",
};

/**
 * A player's overall rating as a chip: the number alone, in its tier's tone. The tier is said in
 * words for a screen reader and as a tooltip ("Overall 79, tier A"), so the colour is never the
 * only signal. It replaces each screen's own OVR badge as the pages move onto it.
 */
export function RatingChip({ value, tier, size = "sm", name }: RatingChipProps) {
  const { t } = useTranslation();
  const rating = Math.round(value);
  const letterKey = ratingTierLetterKey(tier);
  let label: string;
  if (name === undefined) {
    label = letterKey
      ? t("common.overallRatingTier", { value: rating, tier: t(letterKey) })
      : t("common.overallRating", { value: rating });
  } else {
    label = letterKey
      ? t("common.namedRatingTier", { name, value: rating, tier: t(letterKey) })
      : t("common.namedRating", { name, value: rating });
  }
  return (
    <span
      title={label}
      className={`inline-flex shrink-0 items-center justify-center rounded font-heading font-bold tabular-nums ${SIZE[size]} ${ratingTierClass(tier, "badge")}`}
    >
      <span aria-hidden="true">{rating}</span>
      <span className="sr-only">{label}</span>
    </span>
  );
}
