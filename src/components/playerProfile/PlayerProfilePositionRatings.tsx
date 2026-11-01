import { formatDate } from "../../lib/dateFormatting";
import type { Familiarity } from "../../store/generated/Familiarity";
import type { PositionRating } from "../../store/generated/PositionRating";
import type { PositionRatings } from "../../store/generated/PositionRatings";
import { Badge } from "../ui";

type TranslateFn = (key: string, options?: Record<string, string | number>) => string;

interface PlayerProfilePositionRatingsProps {
  ratings: PositionRatings;
  locale: string;
  t: TranslateFn;
}

/**
 * How he stands at a position, in the screens' existing words. The tone follows the maintainer's
 * rule (21 Oct): nothing marks his natural position, "can play there" is a quiet cue and only "out
 * of position" carries a warning, and a modest one. Never a verdict on how well he fits.
 */
function familiarityWords(familiarity: Familiarity, t: TranslateFn): string {
  switch (familiarity) {
    case "natural":
      return t("squad.naturalFit");
    case "can_play":
      return t("squad.adaptedFit");
    case "out_of_position":
      return t("squad.outOfPosition");
  }
}

function familiarityTone(familiarity: Familiarity): string {
  return familiarity === "out_of_position" ? "text-warning" : "text-fg-muted";
}

function PositionRow({ row, t }: { row: PositionRating; t: TranslateFn }) {
  const positionName = t(`common.positions.${row.position}`);
  return (
    <li className="py-1.5">
      <p className="flex flex-wrap items-baseline gap-x-2 text-body">
        <span className="text-fg">{positionName}</span>
        <span className="font-heading font-bold tabular-nums text-fg">{row.rating}</span>
        <span className={`text-caption ${familiarityTone(row.familiarity)}`}>
          {familiarityWords(row.familiarity, t)}
          {row.deployed ? ` (${t("playerProfile.positions.wherePlayed")})` : null}
        </span>
      </p>
      <p className="text-caption text-fg-muted">
        {row.key_attributes
          .map((key) => `${t(`common.attributes.${key.attribute}`)} ${key.value}`)
          .join(" · ")}
      </p>
    </li>
  );
}

/**
 * What the player is worth at each position he plays, as the backend gives it: his natural
 * position, his alternates and, for the club's own player, where he is played. A scouted player's
 * ratings come from the report, and say so as the attributes card does.
 */
export default function PlayerProfilePositionRatings({
  ratings,
  locale,
  t,
}: PlayerProfilePositionRatingsProps) {
  const title = t("playerProfile.positions.title");
  const { source } = ratings;

  return (
    <section aria-label={title} className="mt-3 rounded-lg bg-sunken px-3 py-2">
      <p className="font-heading text-caption font-bold uppercase tracking-label text-fg-muted">
        {title}
      </p>
      {source.kind === "report" ? (
        <p className="mt-1 flex flex-wrap items-center gap-2 text-caption text-fg-muted">
          <span>
            {t("playerProfile.scoutedOn", { date: formatDate(source.scouted_on, locale) })}
          </span>
          {source.current ? null : (
            <Badge variant="accent">{t("playerProfile.scoutReportOutOfDate")}</Badge>
          )}
        </p>
      ) : null}
      <ul className="mt-1 divide-y divide-line">
        {ratings.rows.map((row) => (
          <PositionRow key={row.position} row={row} t={t} />
        ))}
      </ul>
    </section>
  );
}
