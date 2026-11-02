import { useTranslation } from "react-i18next";

import { visibleRatingText } from "../../lib/playerKnowledge";
import type { TeamCard as TeamCardData } from "../../services/teamsService";
import { Badge, CountryFlag, EntityCard, TeamLogo } from "../ui";

interface TeamCardProps {
  card: TeamCardData;
  /** The manager's own club, which the card marks. */
  isUser: boolean;
  onOpen: () => void;
}

/**
 * A club's card in the Teams directory, on the shared card shell: its crest, country and city in
 * the identity row, its league position as the hero, the standing and the squad's average in the
 * status band, and its squad's size and reputation as the two facts.
 *
 * The approved card (round 4, v4) also has the club's last five results and its manager in the
 * status band. Neither is projected yet: the form comes with its own PR, the manager with the
 * manager profile's projection.
 */
export default function TeamCard({ card, isUser, onOpen }: TeamCardProps) {
  const { t, i18n } = useTranslation();
  const { team, standing } = card;
  return (
    <EntityCard
      kit={team.colors}
      media={<TeamLogo team={team} />}
      name={team.name}
      meta={
        <>
          <CountryFlag code={team.country} locale={i18n.language} />
          <span>{team.city}</span>
          {isUser ? (
            <Badge variant="accent" size="sm">
              {t("teams.yourTeam")}
            </Badge>
          ) : null}
        </>
      }
      hero={
        <span>
          <span className="sr-only">{t("common.position")} </span>
          {card.league_pos > 0 ? card.league_pos : "—"}
        </span>
      }
      status={
        <dl className="flex flex-wrap gap-x-4 gap-y-1 tabular-nums">
          {standing ? (
            <>
              <StatusFigure label={t("common.won")} value={standing.won} />
              <StatusFigure label={t("common.drawn")} value={standing.drawn} />
              <StatusFigure label={t("common.lost")} value={standing.lost} />
              <StatusFigure label={t("common.pts")} value={standing.points} />
            </>
          ) : (
            // No table yet: one dash, not a row of zeros that reads as a record.
            <StatusFigure label={t("common.pts")} value="—" />
          )}
          {/* Withheld ("–") for a club whose squad the manager doesn't know. */}
          <StatusFigure label={t("teams.avgOvr")} value={visibleRatingText(card.avg_ovr)} />
        </dl>
      }
      facts={[
        { label: t("teams.squad"), value: card.roster_size },
        { label: t("teams.rep"), value: team.reputation },
      ]}
      onOpen={onOpen}
    />
  );
}

function StatusFigure({ label, value }: { label: string; value: string | number }) {
  return (
    <div className="flex items-baseline gap-1">
      <dt className="text-caption text-fg-muted">{label}</dt>
      <dd className="font-semibold">{value}</dd>
    </div>
  );
}
