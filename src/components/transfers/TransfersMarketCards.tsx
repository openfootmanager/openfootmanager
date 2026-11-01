import { useRatingTiers } from "../../hooks/useRatingTiers";
import { useTranslation } from "react-i18next";

import { formatVal, formatWeeklyWage } from "../../lib/helpers";
import { visibleWageText } from "../../lib/playerKnowledge";
import { naturalPositionOf } from "../../lib/positions";
import type { PlayerData } from "../../store/gameStore";
import ContextMenu from "../ContextMenu";
import PlayerCard from "../PlayerCard";
import { Badge, CardGrid } from "../ui";
import { PlayerOffersList } from "./TransfersPlayerOffers";
import { TransferDealButton, transferPlayerRowModel } from "./TransfersPlayerRow";
import type { TransfersMarketTableState } from "./TransfersMarketTable";

type TransferPlayerCardProps = { state: TransfersMarketTableState; player: PlayerData };

/**
 * One market player as a card: what his table row shows (value and the wage the club may see,
 * rating, listing, offers, the deal button), arranged as the shared player card. The row's own
 * model supplies the age, the offers and the context menu, so the two views stay one.
 */
function TransferPlayerCard({ state, player }: TransferPlayerCardProps) {
  const { t } = useTranslation();
  const tierOf = useRatingTiers();
  const { view, isScoutingView, gameState, onSelectPlayer, onSelectTeam } = state;
  const model = transferPlayerRowModel(state, player);
  const { figures } = model;
  const team = player.team_id ? gameState.teams.find((tm) => tm.id === player.team_id) : undefined;
  const showOffers = view === "offers";
  const showDeal = isScoutingView || view === "offers";
  const hasStatus = player.transfer_listed || player.loan_listed || showOffers || showDeal;
  return (
    <ContextMenu items={model.contextItems}>
      <div className="contents">
        <PlayerCard
          portrait={player}
          name={player.full_name}
          position={naturalPositionOf(player)}
          nationality={player.nationality}
          age={model.age}
          teamName={player.team_id ? (team?.name ?? "") : t("common.freeAgent")}
          onSelectTeam={player.team_id ? () => onSelectTeam(player.team_id as string) : undefined}
          ovr={figures.ovr}
          tier={figures.ovr === null ? null : tierOf(figures.ovr)}
          kit={team?.colors ?? null}
          status={
            hasStatus ? (
              <div className="flex flex-col gap-2">
                {player.transfer_listed || player.loan_listed ? (
                  <div className="flex flex-wrap gap-1.5">
                    {player.transfer_listed && (
                      <Badge variant="accent" size="sm">
                        {t("transfers.transfer")}
                      </Badge>
                    )}
                    {player.loan_listed && (
                      <Badge variant="primary" size="sm">
                        {t("transfers.loan")}
                      </Badge>
                    )}
                  </div>
                ) : null}
                {showOffers ? <PlayerOffersList state={state} player={player} {...model} /> : null}
                {showDeal ? (
                  <div>
                    <TransferDealButton state={state} player={player} />
                  </div>
                ) : null}
              </div>
            ) : null
          }
          facts={[
            { label: t("common.value"), value: formatVal(player.market_value) },
            {
              label: t("common.wage"),
              value: visibleWageText(
                figures.wage,
                (weekly) => formatWeeklyWage(weekly, "compact"),
                t,
              ),
            },
          ]}
          onOpen={() => onSelectPlayer(player.id)}
        />
      </div>
    </ContextMenu>
  );
}

/** The market list as cards, one per player on the page. */
export function TransfersMarketCards({ state }: { state: TransfersMarketTableState }) {
  return (
    <CardGrid>
      {state.visibleList.map((player) => (
        <li key={player.id} className="min-w-0">
          <TransferPlayerCard state={state} player={player} />
        </li>
      ))}
    </CardGrid>
  );
}
