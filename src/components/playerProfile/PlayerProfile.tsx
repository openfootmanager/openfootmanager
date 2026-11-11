import { useState } from "react";
import { useTranslation } from "react-i18next";

import { useContractRiskLevels } from "../../hooks/useContractWarningStages";
import { getPlayerOvr } from "../../lib/helpers";
import { naturalPositionOf } from "../../lib/positions";
import { calcAgeOnDate } from "../../lib/valueFormatting";
import type { GameStateData, PlayerData } from "../../store/gameStore";
import { getPlayerTeamName } from "./PlayerProfile.helpers";
import { buildPlayerProfileRelationship } from "./PlayerProfile.viewModel";
import PlayerProfileActionsMenu from "./PlayerProfileActionsMenu";
import PlayerProfileInjuryBanner from "./PlayerProfileInjuryBanner";
import PlayerProfileLoanStatusBanner from "./PlayerProfileLoanStatusBanner";
import PlayerProfileModals from "./PlayerProfileModals";
import PlayerProfileNameCard from "./PlayerProfileNameCard";
import PlayerProfileTabs, { type ProfileTab } from "./PlayerProfileTabs";
import { retirementOf } from "./retirement";
import { usePlayerProfileActions } from "./usePlayerProfileActions";
import { usePlayerProfileData } from "./usePlayerProfileData";
import { usePlayerProfileKnowledge } from "./usePlayerProfileKnowledge";

interface PlayerProfileProps {
  player: PlayerData;
  gameState: GameStateData;
  isOwnClub: boolean;
  startWithRenewalModal?: boolean;
  startWithTerminationModal?: boolean;
  onSelectTeam?: (id: string) => void;
  onGameUpdate?: (g: GameStateData) => void;
}

/**
 * A player's profile: his name card, the banners that need saying (loan, a failed action, injury),
 * the four tabs and the dialogs. It composes the data and the actions the hooks hold; each tab's
 * content is in `PlayerProfileTabs`.
 */
export default function PlayerProfile({
  player,
  gameState,
  isOwnClub,
  startWithRenewalModal = false,
  startWithTerminationModal = false,
  onSelectTeam,
  onGameUpdate,
}: PlayerProfileProps) {
  const { t, i18n } = useTranslation();
  // The page reuses this instance when another player is opened, so the tab is reset on every
  // change of player (A to B and back to A included), not remembered per player.
  const [tab, setTab] = useState<ProfileTab>("overview");
  const [shownPlayerId, setShownPlayerId] = useState(player.id);
  if (shownPlayerId !== player.id) {
    setShownPlayerId(player.id);
    setTab("overview");
  }

  const data = usePlayerProfileData({ player, currentDate: gameState.clock.current_date });
  const relationship = buildPlayerProfileRelationship(player, gameState, isOwnClub);
  const { managerTeamId, isManagerOwnedProfile, isManagerSquadProfile, isFreeAgent } = relationship;
  const knowledge = usePlayerProfileKnowledge({ player, gameState, isManagerSquadProfile });
  const actions = usePlayerProfileActions({
    player,
    gameState,
    isManagerOwnedProfile,
    hasAssistantManager: relationship.hasAssistantManager,
    startWithRenewalModal,
    startWithTerminationModal,
    onGameUpdate,
  });

  // A retired player has no club and nothing to scout, sign or renew: the card says he retired.
  const retirement = retirementOf(player);
  const contractRiskLevel = useContractRiskLevels(
    [player],
    gameState.clock.current_date,
  )(player.id);
  const contractRiskLabel =
    contractRiskLevel === "critical"
      ? t("finances.contractRiskCritical")
      : contractRiskLevel === "warning"
        ? t("finances.contractRiskWarning")
        : t("finances.contractRiskStable");
  const { contract, scout, renewal, freeAgent, bid } = actions;

  return (
    <div>
      <PlayerProfileNameCard
        player={player}
        ovr={knowledge.ratingKnown ? getPlayerOvr(player) : null}
        primaryPosition={naturalPositionOf(player)}
        age={calcAgeOnDate(player.date_of_birth, gameState.clock.current_date)}
        teamName={getPlayerTeamName(gameState.teams, player.team_id, {
          freeAgent: t("common.freeAgent"),
          unknown: t("common.unknown"),
        })}
        footednessLabel={t(`common.footedness.${player.footedness || "Right"}`)}
        weakFootValue={player.weak_foot ?? 2}
        language={i18n.language}
        isOwnClub={isManagerSquadProfile || !onGameUpdate}
        scoutAvailability={scout.scoutAvailability}
        scoutStatus={scout.scoutStatus}
        scoutError={scout.scoutError}
        onScout={scout.sendScout}
        onSelectTeam={onSelectTeam}
        team={actions.playerTeam}
        retirement={retirement}
        actions={
          onGameUpdate && !retirement ? (
            <PlayerProfileActionsMenu
              player={player}
              gameState={gameState}
              isManagerOwnedProfile={isManagerOwnedProfile}
              isFreeAgent={isFreeAgent}
              hasLetExpireIntent={relationship.hasLetExpireIntent}
              contractRiskLevel={contractRiskLevel}
              actionSubmitting={contract.contractActionSubmitting}
              onGameUpdate={onGameUpdate}
              onOpenRenewal={renewal.openRenewalModal}
              onMarkLetExpire={() => void contract.handleMarkLetExpire()}
              onClearLetExpire={() => void contract.handleClearLetExpire()}
              onOpenTermination={() => void contract.openTerminationModal()}
              onOpenBid={() => bid.openBidNegotiation(player)}
              onOpenFreeAgentContract={() => freeAgent.openFreeAgentContract(player)}
              onError={contract.setContractActionError}
            />
          ) : null
        }
        t={t}
      />

      {player.active_loan ? (
        <PlayerProfileLoanStatusBanner
          loan={player.active_loan}
          teams={gameState.teams}
          managerTeamId={managerTeamId}
          language={i18n.language}
          t={t}
        />
      ) : null}

      {/* Outside the tab panels: the Actions menu in the name card starts these, from any tab. */}
      {contract.contractActionError ? (
        <div
          role="alert"
          className="mb-4 rounded-lg border border-line bg-danger-tint px-4 py-3 text-body text-danger"
        >
          {contract.contractActionError}
        </div>
      ) : null}

      {/* Injury banner */}
      {player.injury ? <PlayerProfileInjuryBanner injury={player.injury} t={t} /> : null}

      <PlayerProfileTabs
        tab={tab}
        onTabChange={setTab}
        player={player}
        currentDate={gameState.clock.current_date}
        canSetTacticalRole={isOwnClub && Boolean(onGameUpdate)}
        relationship={relationship}
        data={data}
        knowledge={knowledge}
        actions={actions}
        contractRisk={{ level: contractRiskLevel, label: contractRiskLabel }}
      />

      <PlayerProfileModals
        player={player}
        teams={gameState.teams}
        actions={actions}
        hasAssistantManager={relationship.hasAssistantManager}
      />
    </div>
  );
}
