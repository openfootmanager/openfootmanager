import { formatDate } from "../../lib/dateFormatting";
import { ratingTierClass } from "../../lib/ratingTier";
import type { Certainty } from "../../store/generated/Certainty";
import type { PlayerTier } from "../../store/generated/PlayerTier";
import type { PotentialOpinion } from "../../store/generated/PotentialOpinion";
import { Badge } from "../ui";

type TranslateFn = (key: string, options?: Record<string, string | number>) => string;

interface PlayerProfilePotentialOpinionProps {
  opinion: PotentialOpinion;
  locale: string;
  t: TranslateFn;
}

/** What the band's middle says about where he could go: the backend's tier, in words. */
function bandWords(band: PlayerTier, t: TranslateFn): string {
  switch (band) {
    case "S":
      return t("playerProfile.potentialOpinion.bandS");
    case "A":
      return t("playerProfile.potentialOpinion.bandA");
    case "B":
      return t("playerProfile.potentialOpinion.bandB");
    case "Rest":
      return t("playerProfile.potentialOpinion.bandRest");
  }
}

/** How sure the judge is, one word per judging band (maintainer, B3-Q1). */
function certaintyWords(certainty: Certainty, t: TranslateFn): string {
  switch (certainty) {
    case "confident":
      return t("playerProfile.potentialOpinion.confident");
    case "fairly_sure":
      return t("playerProfile.potentialOpinion.fairlySure");
    case "unsure":
      return t("playerProfile.potentialOpinion.unsure");
    case "guess":
      return t("playerProfile.potentialOpinion.guess");
  }
}

/**
 * The club's opinion of a player's potential, as the backend gives it: a range and what it says
 * about him, never the number. A scout's opinion names the scout and his report's date, and an
 * old report says it is out of date.
 */
export default function PlayerProfilePotentialOpinion({
  opinion,
  locale,
  t,
}: PlayerProfilePotentialOpinionProps) {
  const { source } = opinion;
  const title =
    source.kind === "scout"
      ? t("playerProfile.potentialOpinion.scoutsOpinion")
      : t("playerProfile.potentialOpinion.coachesOpinion");

  return (
    <section aria-label={title} className="mb-3 rounded-lg bg-sunken px-3 py-2">
      <p className="font-heading text-caption font-bold uppercase tracking-label text-fg-muted">
        {title}
      </p>
      <p
        className={`mt-0.5 font-heading text-lead font-bold ${ratingTierClass(opinion.band, "text")}`}
      >
        {bandWords(opinion.band, t)}
      </p>
      <p className="mt-0.5 flex flex-wrap items-center gap-2 text-body text-fg">
        <span className="tabular-nums">
          {t("playerProfile.potentialOpinion.range", { low: opinion.low, high: opinion.high })}
        </span>
        <span className="text-fg-muted">{certaintyWords(opinion.certainty, t)}</span>
      </p>
      {source.kind === "scout" ? (
        <p className="mt-1 flex flex-wrap items-center gap-2 text-caption text-fg-muted">
          <span>
            {source.scout_name
              ? t("playerProfile.potentialOpinion.byScout", {
                  scout: source.scout_name,
                  date: formatDate(source.scouted_on, locale),
                })
              : t("playerProfile.scoutedOn", { date: formatDate(source.scouted_on, locale) })}
          </span>
          {source.current ? null : (
            <Badge variant="accent">{t("playerProfile.scoutReportOutOfDate")}</Badge>
          )}
        </p>
      ) : null}
    </section>
  );
}
