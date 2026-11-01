import { useTranslation } from "react-i18next";

import { formatVal, formatWeeklyWage } from "../../lib/helpers";
import { visibleWageText } from "../../lib/playerKnowledge";
import { naturalPositionOf } from "../../lib/positions";
import type { RatingTier } from "../../services/squadService";
import type { PlayerSummary } from "../../services/playersService";
import type { TeamColors } from "../../store/types";
import type { ContextMenuItem } from "../ContextMenu";
import ContextMenu from "../ContextMenu";
import PlayerCard from "../PlayerCard";
import { Badge, CardGrid } from "../ui";

interface PlayersCardGridProps {
  items: readonly PlayerSummary[];
  /** The backend's tier for a rating (`useRatingTiers`). */
  tierOf: (ovr: number) => RatingTier;
  /** Each club's kit by id, so a card wears its player's club colours; a free agent has none. */
  kitOf: (teamId: string) => TeamColors | null;
  /** The actions of a player's context menu, the same as his table row's. */
  contextItemsFor: (summary: PlayerSummary) => ContextMenuItem[];
  onSelectPlayer: (id: string) => void;
  onSelectTeam: (id: string) => void;
}

/**
 * The Players list as cards: one per player on the page, with his value and the wage the club may
 * see as the two facts and his listing and injury as the status. What he shows is what his table
 * row shows; only the shape differs.
 */
export default function PlayersCardGrid({
  items,
  tierOf,
  kitOf,
  contextItemsFor,
  onSelectPlayer,
  onSelectTeam,
}: PlayersCardGridProps) {
  const { t } = useTranslation();
  return (
    <CardGrid>
      {items.map((summary) => {
        const hasStatus = summary.transfer_listed || summary.loan_listed || summary.injured;
        return (
          <li key={summary.id} className="min-w-0">
            <ContextMenu items={contextItemsFor(summary)}>
              <div className="contents">
                <PlayerCard
                  portrait={summary}
                  name={summary.match_name}
                  position={naturalPositionOf(summary)}
                  nationality={summary.nationality}
                  age={summary.age}
                  teamName={summary.team_id ? (summary.team_name ?? "") : t("common.freeAgent")}
                  onSelectTeam={
                    summary.team_id ? () => onSelectTeam(summary.team_id as string) : undefined
                  }
                  ovr={summary.ovr}
                  tier={summary.ovr === null ? null : tierOf(summary.ovr)}
                  kit={summary.team_id ? kitOf(summary.team_id) : null}
                  status={
                    hasStatus ? (
                      <div className="flex flex-wrap gap-1.5">
                        {summary.transfer_listed && (
                          <Badge variant="accent" size="sm">
                            {t("transfers.transfer")}
                          </Badge>
                        )}
                        {summary.loan_listed && (
                          <Badge variant="primary" size="sm">
                            {t("transfers.loan")}
                          </Badge>
                        )}
                        {summary.injured && (
                          <Badge variant="danger" size="sm">
                            {t("common.injured")}
                          </Badge>
                        )}
                      </div>
                    ) : null
                  }
                  facts={[
                    { label: t("common.value"), value: formatVal(summary.market_value) },
                    {
                      label: t("common.wage"),
                      value: visibleWageText(
                        summary.wage,
                        (weekly) => formatWeeklyWage(weekly, "compact"),
                        t,
                      ),
                    },
                  ]}
                  onOpen={() => onSelectPlayer(summary.id)}
                />
              </div>
            </ContextMenu>
          </li>
        );
      })}
    </CardGrid>
  );
}
