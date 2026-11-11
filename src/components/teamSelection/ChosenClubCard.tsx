import { Shield } from "lucide-react";
import { useId } from "react";
import { useTranslation } from "react-i18next";

import { useRatingTiers } from "../../hooks/useRatingTiers";
import { competitionDisplayName } from "../../lib/competitionName";
import { formatVal, formatWeeklyWage } from "../../lib/helpers";
import type { ClubSquadRatings } from "../../services/teamsService";
import type { TeamData } from "../../store/gameStore";
import { resolveBackendText } from "../../utils/backendI18n";
import { translatePositionAbbreviation, translatePositionLabel } from "../squad/SquadTab.helpers";
import { EntityCard, PositionBadge, RatingChip, TeamLocation, TeamLogo } from "../ui";

/** What a figure the backend has not given yet reads as. */
const NOT_YET = "\u2014";

/** A dash read aloud as what it stands for: a figure the backend has not given yet. */
function NotYet({ unknownLabel }: { unknownLabel: string }) {
  return (
    <>
      <span aria-hidden="true">{NOT_YET}</span>
      <span className="sr-only">{unknownLabel}</span>
    </>
  );
}

interface ChosenClubCardProps {
  /** The club chosen, or null while none is. */
  team: TeamData | null;
  /** The club's average as the backend rates it; null until it answers. */
  average: number | null;
  /** The club's facts as the backend gives them; undefined until it answers. */
  facts: ClubSquadRatings | undefined;
  /** A player's age on the game's date; null for one the game state does not hold. */
  ageOf: (playerId: string) => number | null;
}

function Fact({ label, value }: { label: string; value: string }) {
  return (
    <div className="flex min-h-8 items-center justify-between gap-3 border-b border-line last:border-b-0">
      <dt className="text-caption text-fg-muted">{label}</dt>
      <dd className="text-right font-semibold tabular-nums">{value}</dd>
    </div>
  );
}

/**
 * The club the player has chosen, as a card: its kit, name and squad rating, then what the board
 * expects, its reputation, its budgets and stadium, its five key players and its competitions. Every
 * fact is the backend's (`get_team_selection_ratings`): the reputation word and the expectation are
 * translation keys it names, and the card judges no number. Until it answers they read as a dash.
 */
export default function ChosenClubCard({ team, average, facts, ageOf }: ChosenClubCardProps) {
  const { t, i18n } = useTranslation();
  const tierOf = useRatingTiers();
  const keyPlayersId = useId();

  if (!team) {
    return (
      <div className="flex flex-1 flex-col items-center justify-center gap-2.5 rounded-xl border border-dashed border-line-strong bg-panel p-6 text-center">
        <Shield aria-hidden="true" className="size-10 text-fg-subtle" />
        <p className="font-heading text-heading font-semibold uppercase tracking-label text-fg-muted">
          {t("teamSelect.card.emptyTitle")}
        </p>
        <p className="max-w-64 text-body text-fg-subtle">{t("teamSelect.card.emptyHint")}</p>
      </div>
    );
  }

  return (
    <EntityCard
      kit={team.colors}
      media={
        <TeamLogo
          team={team}
          className="grid size-12 shrink-0 place-items-center overflow-hidden"
          imageClassName="size-11 object-contain"
        />
      }
      name={team.name}
      meta={
        <>
          <TeamLocation
            city={team.city}
            countryCode={team.country}
            locale={i18n.language}
            iconClassName="size-3"
            flagClassName="text-caption leading-none"
          />
          <span>{t("teamSelect.card.founded", { year: team.founded_year })}</span>
        </>
      }
      hero={
        average === null ? NOT_YET : <RatingChip value={average} tier={tierOf(average)} size="md" />
      }
      status={
        <div className="flex flex-col gap-3">
          <dl className="grid grid-cols-2 gap-3">
            <div className="flex flex-col gap-0.5">
              <dt className="text-caption text-fg-muted">{t("teamSelect.card.boardExpects")}</dt>
              <dd className="font-semibold">
                {facts ? (
                  resolveBackendText(facts.opening_expectation.label_key, NOT_YET)
                ) : (
                  <NotYet unknownLabel={t("common.unknown")} />
                )}
              </dd>
            </div>
            <div className="flex flex-col gap-0.5">
              <dt className="text-caption text-fg-muted">{t("teamSelect.reputation")}</dt>
              <dd className="font-semibold">
                {facts ? (
                  resolveBackendText(facts.reputation_key, NOT_YET)
                ) : (
                  <NotYet unknownLabel={t("common.unknown")} />
                )}
              </dd>
            </div>
          </dl>
          <dl>
            <Fact
              label={t("teamSelect.card.transferBudget")}
              value={facts ? formatVal(facts.transfer_budget) : NOT_YET}
            />
            <Fact
              label={t("teamSelect.card.wageBudget")}
              value={facts ? formatWeeklyWage(facts.weekly_wage_budget, "compact") : NOT_YET}
            />
            <Fact
              label={t("teamSelect.card.balance")}
              value={facts ? formatVal(facts.balance) : NOT_YET}
            />
            <Fact
              label={t("teamSelect.card.stadium")}
              value={
                facts
                  ? `${facts.stadium_name} \u00b7 ${facts.stadium_capacity.toLocaleString(i18n.language)}`
                  : NOT_YET
              }
            />
          </dl>
          <section aria-labelledby={keyPlayersId}>
            <h3
              id={keyPlayersId}
              className="mb-1 font-heading text-lead font-semibold uppercase tracking-label text-fg-muted"
            >
              {t("teamSelect.keyPlayers")}
            </h3>
            <ul>
              {(facts?.key_players ?? []).map((player) => (
                <li
                  key={player.player_id}
                  className="flex min-h-8 items-center gap-2 border-b border-line last:border-b-0"
                >
                  <PositionBadge
                    position={player.position}
                    label={translatePositionAbbreviation(t, player.position)}
                    name={translatePositionLabel(t, player.position)}
                  />
                  <span className="min-w-0 flex-1 [overflow-wrap:anywhere]">{player.name}</span>
                  {ageOf(player.player_id) === null ? null : (
                    <span className="text-caption tabular-nums text-fg-muted">
                      <span className="sr-only">{t("common.age")} </span>
                      {ageOf(player.player_id)}
                    </span>
                  )}
                  <RatingChip value={player.ovr} tier={tierOf(player.ovr)} />
                </li>
              ))}
            </ul>
          </section>
          <ul aria-label={t("teamSelect.competitions")} className="flex flex-wrap gap-1.5">
            {(facts?.competitions ?? []).map((competition) => (
              <li
                key={competition.id}
                className="rounded-md border border-line bg-sunken px-2 py-0.5 text-caption font-semibold text-fg-muted"
              >
                {competitionDisplayName(competition, t)}
              </li>
            ))}
          </ul>
        </div>
      }
    />
  );
}
