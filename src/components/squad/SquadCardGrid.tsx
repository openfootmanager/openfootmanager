import { useTranslation } from "react-i18next";

import { useRatingTiers } from "../../hooks/useRatingTiers";
import { type ContractRiskLevel, contractExpiryLabel } from "../../lib/contractUtils";
import { formatDate } from "../../lib/dateFormatting";
import { calcAge, formatWeeklyWage, getPlayerOvr } from "../../lib/helpers";
import { visibleWageText } from "../../lib/playerKnowledge";
import { naturalPositionOf } from "../../lib/positions";
import type { RatingTier } from "../../services/squadService";
import type { PlayerData } from "../../store/gameStore";
import type { TeamColors } from "../../store/types";
import type { ContextMenuItem } from "../ContextMenu";
import ContextMenu from "../ContextMenu";
import PlayerCard from "../PlayerCard";
import { contractChip } from "./SquadRosterView.columns";
import { Badge, CardGrid, ConditionMeter, InjuryBadge } from "../ui";

interface SquadCardGridProps {
  players: readonly PlayerData[];
  /** The manager's own club: every card wears its name and kit. */
  teamName: string;
  kit: TeamColors;
  isStarter: (player: PlayerData) => boolean;
  /** The backend's contract warning stage for a player (`useContractRiskLevels`). */
  riskLevelOf: (playerId: string) => ContractRiskLevel;
  /** A player's context menu: the very items his roster row offers. */
  menuItemsFor: (player: PlayerData) => ContextMenuItem[];
  onSelectPlayer: (id: string) => void;
}

/**
 * The squad as cards: one per first-team player, each with his roster row's menu. What a card says
 * is what the row says; only the shape differs.
 */
export default function SquadCardGrid({
  players,
  teamName,
  kit,
  isStarter,
  riskLevelOf,
  menuItemsFor,
  onSelectPlayer,
}: SquadCardGridProps) {
  const tierOf = useRatingTiers();
  return (
    <CardGrid>
      {players.map((player) => (
        <li key={player.id} className="min-w-0">
          <ContextMenu items={menuItemsFor(player)}>
            <div className="contents">
              <SquadPlayerCard
                player={player}
                teamName={teamName}
                kit={kit}
                starting={isStarter(player)}
                risk={riskLevelOf(player.id)}
                tierOf={tierOf}
                onOpen={() => onSelectPlayer(player.id)}
              />
            </div>
          </ContextMenu>
        </li>
      ))}
    </CardGrid>
  );
}

interface SquadPlayerCardProps {
  player: PlayerData;
  teamName: string;
  kit: TeamColors;
  starting: boolean;
  risk: ContractRiskLevel;
  tierOf: (ovr: number) => RatingTier;
  onOpen: () => void;
}

/**
 * One squad player's card: his rating as the hero, his condition, XI place, contract stage and
 * injury as the status, and his wage and the month his contract ends as the two facts.
 */
function SquadPlayerCard({
  player,
  teamName,
  kit,
  starting,
  risk,
  tierOf,
  onOpen,
}: SquadPlayerCardProps) {
  const { t, i18n } = useTranslation();
  const ovr = getPlayerOvr(player);
  return (
    <PlayerCard
      portrait={player}
      name={player.match_name}
      position={naturalPositionOf(player)}
      nationality={player.nationality}
      age={calcAge(player.date_of_birth)}
      teamName={teamName}
      ovr={ovr}
      tier={tierOf(ovr)}
      kit={kit}
      status={<SquadCardStatus player={player} starting={starting} risk={risk} />}
      facts={[
        {
          label: t("common.wage"),
          // His own club pays him, so the club sees his wage as it is.
          value: visibleWageText(
            { kind: "exact", weekly: player.wage },
            (weekly) => formatWeeklyWage(weekly, "compact"),
            t,
          ),
        },
        {
          label: t("common.contract"),
          value: player.contract_end ? (
            <span title={contractExpiryLabel(player.contract_end, i18n.language, t)}>
              {formatDate(player.contract_end, i18n.language, { month: "short", year: "numeric" })}
            </span>
          ) : (
            "—"
          ),
        },
      ]}
      onOpen={onOpen}
    />
  );
}

/**
 * A card's status band: the condition as the backend bands it, then in words that he starts, his
 * contract's one chip and his injury, which the table row says with its accent, chip and badge.
 */
function SquadCardStatus({
  player,
  starting,
  risk,
}: {
  player: PlayerData;
  starting: boolean;
  risk: ContractRiskLevel;
}) {
  const { t } = useTranslation();
  const chip = contractChip(player, risk, t);
  return (
    <>
      <ConditionMeter value={player.condition} status={player.condition_status} />
      <div className="flex flex-wrap items-center gap-1.5">
        {starting ? (
          <Badge variant="primary" size="sm">
            {t("squad.startingXi")}
          </Badge>
        ) : null}
        <Badge variant={chip.variant} size="sm">
          {chip.label}
        </Badge>
        {player.injury ? <InjuryBadge injury={player.injury} /> : null}
      </div>
    </>
  );
}
