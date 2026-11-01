import { MoreVertical } from "lucide-react";
import { type ReactNode, useRef, useState } from "react";
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
import type { ContextMenuHandle, ContextMenuItem } from "../ContextMenu";
import ContextMenu from "../ContextMenu";
import PlayerCard from "../PlayerCard";
import { contractChip } from "./SquadRosterView.columns";
import {
  type SquadTacticalFit,
  translatePositionAbbreviation,
  translatePositionLabel,
} from "./SquadTab.helpers";
import { Badge, CardGrid, ConditionMeter, InjuryBadge, PositionBadge } from "../ui";

/** A starter's slot in the XI and how it suits him. */
export interface DeployedPosition {
  position: string;
  /** Null while the backend has no grade for him: the card then claims nothing. */
  familiarity: SquadTacticalFit | null;
}

interface SquadCardGridProps {
  players: readonly PlayerData[];
  /** The manager's own club: every card wears its name and kit. */
  teamName: string;
  kit: TeamColors;
  isStarter: (player: PlayerData) => boolean;
  /** A starter's slot and how it suits him (the row's own reading); null off the XI. */
  deployedPositionOf: (player: PlayerData) => DeployedPosition | null;
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
  deployedPositionOf,
  riskLevelOf,
  menuItemsFor,
  onSelectPlayer,
}: SquadCardGridProps) {
  const tierOf = useRatingTiers();
  return (
    <CardGrid>
      {players.map((player) => (
        <li key={player.id} className="min-w-0">
          <SquadCardMenu player={player} items={menuItemsFor(player)}>
            {(actions) => (
              <SquadPlayerCard
                player={player}
                teamName={teamName}
                kit={kit}
                starting={isStarter(player)}
                deployed={deployedPositionOf(player)}
                risk={riskLevelOf(player.id)}
                tierOf={tierOf}
                actions={actions}
                onOpen={() => onSelectPlayer(player.id)}
              />
            )}
          </SquadCardMenu>
        </li>
      ))}
    </CardGrid>
  );
}

/**
 * A card's menu, opened by a right-click anywhere on the card or by its actions button, the
 * keyboard's and a touch screen's way in, as the roster row's ⋮ button is.
 */
function SquadCardMenu({
  player,
  items,
  children,
}: {
  player: PlayerData;
  items: ContextMenuItem[];
  children: (actions: ReactNode) => ReactNode;
}) {
  const { t } = useTranslation();
  const menu = useRef<ContextMenuHandle>(null);
  const [menuOpen, setMenuOpen] = useState(false);
  const actions = (
    <button
      type="button"
      onClick={(event) => {
        const rect = event.currentTarget.getBoundingClientRect();
        menu.current?.open(rect.left, rect.bottom + 4);
      }}
      aria-label={t("common.playerActions", { name: player.match_name })}
      aria-haspopup="menu"
      aria-expanded={menuOpen}
      className="ml-auto rounded-md p-1.5 text-fg-subtle transition-colors hover:text-fg focus:outline-none focus-visible:ring-2 focus-visible:ring-inset focus-visible:ring-focus"
    >
      <MoreVertical aria-hidden="true" className="h-4 w-4" />
    </button>
  );
  return (
    <ContextMenu ref={menu} items={items} onOpenChange={setMenuOpen}>
      <div className="contents">{children(actions)}</div>
    </ContextMenu>
  );
}

interface SquadPlayerCardProps {
  player: PlayerData;
  teamName: string;
  kit: TeamColors;
  starting: boolean;
  deployed: DeployedPosition | null;
  risk: ContractRiskLevel;
  tierOf: (ovr: number) => RatingTier;
  /** The button that opens the card's menu without a mouse. */
  actions: ReactNode;
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
  deployed,
  risk,
  tierOf,
  actions,
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
      status={
        <SquadCardStatus
          player={player}
          starting={starting}
          deployed={deployed}
          risk={risk}
          actions={actions}
        />
      }
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
 * A card's status band: the condition as the backend bands it, then in words that he starts (and
 * the slot he starts in, when it isn't his natural one), his contract's one chip, his injury, and
 * that he can't be picked when the backend says so for any other reason. The table row says these
 * with its position badges, accent, chip and badge.
 */
function SquadCardStatus({
  player,
  starting,
  deployed,
  risk,
  actions,
}: {
  player: PlayerData;
  starting: boolean;
  deployed: DeployedPosition | null;
  risk: ContractRiskLevel;
  actions: ReactNode;
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
        {/* A natural slot is the quiet default, as on the row: only a slot that isn't is marked. */}
        {deployed?.familiarity && deployed.familiarity !== "natural" ? (
          <PositionBadge
            position={deployed.position}
            label={translatePositionAbbreviation(t, deployed.position)}
            name={translatePositionLabel(t, deployed.position)}
            familiarity={deployed.familiarity}
          />
        ) : null}
        <Badge variant={chip.variant} size="sm">
          {chip.label}
        </Badge>
        {player.injury ? <InjuryBadge injury={player.injury} /> : null}
        {/* An injury already says why he is out; this is for the backend's other reasons. Only an
            explicit "no": before the squad is fetched the verdict is absent, which means not yet. */}
        {!player.injury && player.match_day_eligible === false ? (
          <Badge variant="danger" size="sm">
            {t("squad.notMatchDayEligible")}
          </Badge>
        ) : null}
        {actions}
      </div>
    </>
  );
}
