import { useId, useState } from "react";
import { getPlayerOvr } from "../../lib/helpers";
import { useContractRiskLevels } from "../../hooks/useContractWarningStages";
import type { PlayerData, GameStateData } from "../../store/gameStore";
import { useTranslation } from "react-i18next";
import { TabPanel, Tabs } from "../ui";
import { getRoleOptions } from "../../lib/playerRoles";
import { getDeployedPosition } from "../squad/SquadTab.helpers";
import { setPlayerRole as setPlayerRoleService } from "../../services/squadService";
import type { PlayerRole } from "../../store/types";
import FreeAgentContractModal from "../transfers/FreeAgentContractModal";
import TransferBidModal from "../transfers/TransferBidModal";
import { useFreeAgentContractFlow } from "../transfers/useFreeAgentContractFlow";
import { useTransferBidFlow } from "../transfers/useTransferBidFlow";
import PlayerProfileActionsMenu from "./PlayerProfileActionsMenu";
import { formatPlayerWage, getPlayerTeamName } from "./PlayerProfile.helpers";
import { retirementOf } from "./retirement";
import { useVisibleWage } from "../../hooks/useVisibleWage";
import { answeredOr } from "../../hooks/useKnowledgeAnswer";
import ProfileLoadFailed from "./ProfileLoadFailed";
import { buildPlayerProfileRelationship } from "./PlayerProfile.viewModel";
import PlayerProfileAdvancedStatsCard from "./PlayerProfileAdvancedStatsCard";
import { buildPlayerAttributeGroups, isGoalkeeper } from "./PlayerProfile.attributes";
import { calcAgeOnDate } from "../../lib/valueFormatting";
import { formatDate } from "../../lib/dateFormatting";
import PlayerProfileAttributesCard from "./PlayerProfileAttributesCard";
import PlayerProfileCareerHistoryCard from "./PlayerProfileCareerHistoryCard";
import PlayerProfileContractCard from "./PlayerProfileContractCard";
import PlayerProfileNameCard from "./PlayerProfileNameCard";
import PlayerProfileOverview from "./overview/PlayerProfileOverview";
import PlayerProfileInjuryBanner from "./PlayerProfileInjuryBanner";
import PlayerProfileLoanStatusBanner from "./PlayerProfileLoanStatusBanner";
import PlayerProfileMovementHistoryCard from "./PlayerProfileMovementHistoryCard";
import PlayerProfileRenewalModal from "./PlayerProfileRenewalModal";
import PlayerProfileTerminationModal from "./PlayerProfileTerminationModal";
import { useScoutedReport } from "./useScoutedReport";
import PlayerProfilePotentialOpinion from "./PlayerProfilePotentialOpinion";
import PlayerProfilePositionRatings from "./PlayerProfilePositionRatings";
import { usePositionRatings } from "../../hooks/usePositionRatings";
import { usePotentialOpinion } from "../../hooks/usePotentialOpinion";
import { useContractActionsFlow } from "./useContractActionsFlow";
import { useInitialModalIntent } from "./useInitialModalIntent";
import { usePlayerProfileData } from "./usePlayerProfileData";
import { useScoutPlayerFlow } from "./useScoutPlayerFlow";
import { useContractRenewalFlow } from "./useContractRenewalFlow";
import { naturalPositionOf } from "../../lib/positions";
import {
  gameKnowledgeRevision,
  HIDDEN_WAGE,
  knowsPlayerRating,
  knowsPrivateState,
  ownLineupRevision,
  visibleTraits,
  visibleWageText,
} from "../../lib/playerKnowledge";
import { usePlayerKnowledge } from "../../hooks/usePlayerKnowledge";

/** The profile's sections, in tab order. */
const PROFILE_TABS = ["overview", "attributes", "career", "contract"] as const;
type ProfileTab = (typeof PROFILE_TABS)[number];

interface PlayerProfileProps {
  player: PlayerData;
  gameState: GameStateData;
  isOwnClub: boolean;
  startWithRenewalModal?: boolean;
  startWithTerminationModal?: boolean;
  onSelectTeam?: (id: string) => void;
  onGameUpdate?: (g: GameStateData) => void;
}

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
  const tabsId = useId();
  // The page reuses this instance when another player is opened, so the tab is reset on every
  // change of player (A to B and back to A included), not remembered per player.
  const [tab, setTab] = useState<ProfileTab>("overview");
  const [shownPlayerId, setShownPlayerId] = useState(player.id);
  if (shownPlayerId !== player.id) {
    setShownPlayerId(player.id);
    setTab("overview");
  }
  const primaryPosition = naturalPositionOf(player);
  const footednessLabel = t(`common.footedness.${player.footedness || "Right"}`);
  const weakFootValue = player.weak_foot ?? 2;

  const { advancedStats, recentMatches, recentMatchesStatus, retryRecentMatches } =
    usePlayerProfileData({
      player,
      currentDate: gameState.clock.current_date,
    });
  const { scoutAvailability, scoutStatus, scoutError, sendScout } = useScoutPlayerFlow({
    player,
    gameState,
    onGameUpdate,
  });
  const ovr = getPlayerOvr(player);
  const age = calcAgeOnDate(player.date_of_birth, gameState.clock.current_date);
  const revision = gameKnowledgeRevision(gameState, {
    playerClubId: player.team_id,
    wage: player.wage,
  });
  const knowledge = usePlayerKnowledge([player.id], revision)(player.id);
  // Each answer is loading, answered or failed; a failed one shows "Couldn't load" and a retry,
  // never the empty state that means the club knows nothing (#1108).
  const wageAnswer = useVisibleWage(player.id, revision);
  const visibleWage = answeredOr(wageAnswer, HIDDEN_WAGE);
  const wageRetry = wageAnswer.status === "failed" ? wageAnswer.retry : undefined;
  // The club's opinion of his potential: a range from his scout or his coaches, never the number.
  const opinionAnswer = usePotentialOpinion(player.id, revision);
  const potentialOpinion = answeredOr(opinionAnswer, null);
  // The slot he is played in moves with the lineup, which the knowledge revision does not follow.
  const ratingsAnswer = usePositionRatings(
    player.id,
    `${revision}|${ownLineupRevision(gameState)}`,
  );
  const positionRatings = answeredOr(ratingsAnswer, null);
  const showsPrivateState = knowsPrivateState(knowledge);
  const playerTeam = gameState.teams.find((team) => team.id === player.team_id);
  const currentTacticalRole: PlayerRole = playerTeam?.player_roles?.[player.id] ?? "Standard";
  // Offer roles for the deployed slot when the player is in the starting XI —
  // the backend validates set_player_role against it, so natural-position
  // roles would be rejected for an out-of-position starter (issue #272).
  const roleValidationPosition =
    (playerTeam && getDeployedPosition(playerTeam, player.id)) || primaryPosition;
  const tacticalRoleOptions = getRoleOptions(roleValidationPosition, currentTacticalRole);
  // A retired player has no club and nothing to scout, sign or renew: the card says he retired.
  const retirement = retirementOf(player);
  const teamName = getPlayerTeamName(gameState.teams, player.team_id, {
    freeAgent: t("common.freeAgent"),
    unknown: t("common.unknown"),
  });
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
  const {
    managerTeamId,
    isManagerOwnedProfile,
    isManagerSquadProfile,
    isFreeAgent,
    hasLetExpireIntent,
    hasAssistantManager,
  } = buildPlayerProfileRelationship(player, gameState, isOwnClub);

  const {
    freeAgentTarget,
    contractWage,
    setContractWage,
    contractLength,
    setContractLength,
    contractFeedback,
    contractProjection,
    contractSubmitting,
    contractSubmitDisabled,
    contractStatusMessage,
    contractStatusClassName,
    openFreeAgentContract,
    closeFreeAgentContract,
    submitFreeAgentContract,
  } = useFreeAgentContractFlow({ gameState, onGameUpdate });

  const {
    bidTarget,
    bidAmount,
    setBidAmount,
    bidResult,
    bidLoading,
    bidFeedback,
    bidProjection,
    bidFee,
    activeBidOffer,
    myTeam,
    hasExistingOffer,
    talksResumeOn,
    bidSubmitDisabled,
    openBidNegotiation,
    closeBidNegotiation,
    handleMakeBid,
  } = useTransferBidFlow({ gameState, onGameUpdate });

  const {
    showRenewalModal,
    renewalWage,
    setRenewalWage,
    renewalLength,
    setRenewalLength,
    renewalSubmitting,
    renewalIsTerminal,
    renewalCooledOff,
    renewalFeedback,
    renewalProjection,
    isRenewalWageValid,
    renewalViolatesSoftCap,
    renewalSubmitDisabled,
    renewalStatusMessage,
    renewalStatusClassName,
    openRenewalModal,
    closeRenewalModal,
    handleRenewalSubmit,
    handleDelegateRenewal,
  } = useContractRenewalFlow({
    player,
    gameState,
    hasAssistantManager,
    onGameUpdate,
  });
  const scoutedReport = useScoutedReport(
    player.id,
    !isManagerSquadProfile,
    gameState.clock.current_date,
  );
  const attributesPlayer = scoutedReport
    ? { ...player, attributes: scoutedReport.attributes }
    : player;
  const attrGroups = buildPlayerAttributeGroups(attributesPlayer, t);

  const {
    contractActionSubmitting,
    contractActionError,
    setContractActionError,
    terminationPreview,
    showTerminationModal,
    handleMarkLetExpire,
    handleClearLetExpire,
    openTerminationModal,
    closeTerminationModal,
    handleTerminateContract,
  } = useContractActionsFlow({ player, onGameUpdate });

  // Both modals can be asked for on arrival, when the player reaches this
  // profile from an inbox action. One shared reset key, so switching player
  // re-arms both.
  const modalIntentResetKey = `${player.id}|${startWithRenewalModal}|${startWithTerminationModal}`;

  useInitialModalIntent({
    requested: startWithRenewalModal,
    allowed: isManagerOwnedProfile,
    isOpen: showRenewalModal,
    open: openRenewalModal,
    resetKey: modalIntentResetKey,
  });

  useInitialModalIntent({
    requested: startWithTerminationModal,
    allowed: isManagerOwnedProfile,
    isOpen: showTerminationModal,
    open: () => void openTerminationModal(),
    resetKey: modalIntentResetKey,
  });

  async function handleTacticalRoleChange(role: PlayerRole): Promise<void> {
    if (!onGameUpdate) return;
    try {
      const updated = await setPlayerRoleService(player.id, role);
      onGameUpdate(updated);
    } catch (error) {
      console.error("Failed to set player role:", error);
    }
  }

  const traits = visibleTraits(player.traits ?? [], potentialOpinion);
  const tabItems = PROFILE_TABS.map((id) => ({ id, label: t(`playerProfile.tabs.${id}`) }));
  const hidden = (id: ProfileTab) => tab !== id;

  return (
    <div>
      <PlayerProfileNameCard
        player={player}
        ovr={knowsPlayerRating(knowledge) ? ovr : null}
        primaryPosition={primaryPosition}
        age={age}
        teamName={teamName}
        footednessLabel={footednessLabel}
        weakFootValue={weakFootValue}
        language={i18n.language}
        isOwnClub={isManagerSquadProfile || !onGameUpdate}
        scoutAvailability={scoutAvailability}
        scoutStatus={scoutStatus}
        scoutError={scoutError}
        onScout={sendScout}
        onSelectTeam={onSelectTeam}
        team={playerTeam}
        retirement={retirement}
        actions={
          onGameUpdate && !retirement ? (
            <PlayerProfileActionsMenu
              player={player}
              gameState={gameState}
              isManagerOwnedProfile={isManagerOwnedProfile}
              isFreeAgent={isFreeAgent}
              hasLetExpireIntent={hasLetExpireIntent}
              contractRiskLevel={contractRiskLevel}
              actionSubmitting={contractActionSubmitting}
              onGameUpdate={onGameUpdate}
              onOpenRenewal={openRenewalModal}
              onMarkLetExpire={() => void handleMarkLetExpire()}
              onClearLetExpire={() => void handleClearLetExpire()}
              onOpenTermination={() => void openTerminationModal()}
              onOpenBid={() => openBidNegotiation(player)}
              onOpenFreeAgentContract={() => openFreeAgentContract(player)}
              onError={setContractActionError}
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
      {contractActionError ? (
        <div
          role="alert"
          className="mb-4 rounded-lg border border-line bg-danger-tint px-4 py-3 text-body text-danger"
        >
          {contractActionError}
        </div>
      ) : null}

      {/* Injury banner */}
      {player.injury ? <PlayerProfileInjuryBanner injury={player.injury} t={t} /> : null}

      <div className="mb-4 border-b border-line">
        <Tabs
          idPrefix={tabsId}
          label={t("playerProfile.sections")}
          tabs={tabItems}
          selectedId={tab}
          onSelect={(id) => setTab(id as ProfileTab)}
        />
      </div>

      <TabPanel idPrefix={tabsId} tabId="overview" hidden={hidden("overview")}>
        <PlayerProfileOverview
          player={player}
          language={i18n.language}
          showsPrivateState={showsPrivateState}
          recentMatches={recentMatches}
          recentMatchesStatus={recentMatchesStatus}
          retryRecentMatches={retryRecentMatches}
          seasonRatingRated={advancedStats.seasonRatingRated}
          tacticalRole={
            isOwnClub && onGameUpdate
              ? {
                  value: currentTacticalRole,
                  options: tacticalRoleOptions,
                  onChange: (role) => void handleTacticalRoleChange(role as PlayerRole),
                }
              : null
          }
          contractRiskLevel={contractRiskLevel}
          contractRiskLabel={contractRiskLabel}
          wageText={visibleWageText(visibleWage, formatPlayerWage, t)}
          wageRetry={wageRetry}
          potentialOpinion={
            opinionAnswer.status === "failed" ? (
              <ProfileLoadFailed t={t} onRetry={opinionAnswer.retry} />
            ) : potentialOpinion ? (
              <PlayerProfilePotentialOpinion
                opinion={potentialOpinion}
                locale={i18n.language}
                t={t}
              />
            ) : null
          }
          ratings={ratingsAnswer}
          attributes={
            knowsPlayerRating(knowledge) || scoutedReport !== null
              ? attributesPlayer.attributes
              : null
          }
          traits={traits}
          onOpenTab={setTab}
        />
      </TabPanel>

      <TabPanel idPrefix={tabsId} tabId="attributes" hidden={hidden("attributes")}>
        <PlayerProfileAttributesCard
          attrGroups={attrGroups}
          player={attributesPlayer}
          attributesVisible={knowsPlayerRating(knowledge) || scoutedReport !== null}
          scoutNote={
            scoutedReport
              ? {
                  scoutedLabel: t("playerProfile.scoutedOn", {
                    date: formatDate(scoutedReport.scouted_on, i18n.language),
                  }),
                  outOfDate: scoutedReport.out_of_date,
                  outOfDateLabel: t("playerProfile.scoutReportOutOfDate"),
                }
              : undefined
          }
          isGk={isGoalkeeper(player)}
          title={t("playerProfile.attributes")}
          averageLabel={t("common.average")}
          hiddenTitle={t("playerProfile.attributesHidden")}
          hiddenBody={t("playerProfile.scoutToView")}
          listLabel={t("common.listView")}
          radarLabel={t("common.radarView")}
          potentialOpinion={
            opinionAnswer.status === "failed" ? (
              <ProfileLoadFailed t={t} onRetry={opinionAnswer.retry} />
            ) : potentialOpinion ? (
              <PlayerProfilePotentialOpinion
                opinion={potentialOpinion}
                locale={i18n.language}
                t={t}
              />
            ) : null
          }
          positionRatings={
            ratingsAnswer.status === "failed" ? (
              <ProfileLoadFailed t={t} onRetry={ratingsAnswer.retry} />
            ) : positionRatings ? (
              <PlayerProfilePositionRatings
                ratings={positionRatings}
                locale={i18n.language}
                t={t}
              />
            ) : null
          }
        />
      </TabPanel>

      <TabPanel
        idPrefix={tabsId}
        tabId="career"
        hidden={hidden("career")}
        className="grid grid-cols-1 gap-5"
      >
        <PlayerProfileAdvancedStatsCard summary={advancedStats} t={t} />

        <PlayerProfileCareerHistoryCard career={player.career} t={t} />

        <PlayerProfileMovementHistoryCard
          movementHistory={player.movement_history ?? []}
          showsWages={visibleWage.kind === "exact"}
          t={t}
        />
      </TabPanel>

      <TabPanel
        idPrefix={tabsId}
        tabId="contract"
        hidden={hidden("contract")}
        className="flex flex-col gap-5"
      >
        <PlayerProfileContractCard
          contractEnd={player.contract_end}
          currentDate={gameState.clock.current_date}
          isOwnClub={isManagerOwnedProfile}
          isFreeAgent={isFreeAgent}
          hasLetExpireIntent={hasLetExpireIntent}
          actionSubmitting={contractActionSubmitting}
          onOpenRenewal={openRenewalModal}
          onMarkLetExpire={() => void handleMarkLetExpire()}
          onClearLetExpire={() => void handleClearLetExpire()}
          onOpenTermination={() => void openTerminationModal()}
          onOpenFreeAgentContract={() => openFreeAgentContract(player)}
          t={t}
        />
      </TabPanel>

      {bidTarget && (
        <TransferBidModal
          bidTarget={bidTarget}
          teams={gameState.teams}
          bidAmount={bidAmount}
          onBidAmountChange={setBidAmount}
          myTeam={myTeam}
          bidFee={bidFee}
          bidProjection={bidProjection}
          bidFeedback={bidFeedback}
          activeBidOffer={activeBidOffer}
          hasExistingOffer={hasExistingOffer}
          talksResumeOn={talksResumeOn}
          bidResult={bidResult}
          bidLoading={bidLoading}
          bidSubmitDisabled={bidSubmitDisabled}
          onSubmit={handleMakeBid}
          onClose={closeBidNegotiation}
        />
      )}
      {freeAgentTarget && (
        <FreeAgentContractModal
          player={freeAgentTarget}
          teams={gameState.teams}
          wage={contractWage}
          onWageChange={setContractWage}
          contractLength={contractLength}
          onContractLengthChange={setContractLength}
          projection={contractProjection}
          feedback={contractFeedback}
          statusMessage={contractStatusMessage(t)}
          statusClassName={contractStatusClassName}
          submitting={contractSubmitting}
          submitDisabled={contractSubmitDisabled}
          onSubmit={submitFreeAgentContract}
          onClose={closeFreeAgentContract}
        />
      )}

      <PlayerProfileRenewalModal
        show={showRenewalModal}
        playerName={player.full_name}
        t={t}
        renewalWage={renewalWage}
        renewalLength={renewalLength}
        renewalIsTerminal={renewalIsTerminal}
        isRenewalWageValid={isRenewalWageValid}
        renewalViolatesSoftCap={renewalViolatesSoftCap}
        renewalProjection={renewalProjection}
        renewalStatusMessage={renewalStatusMessage}
        renewalStatusClassName={renewalStatusClassName}
        renewalCooledOff={renewalCooledOff}
        renewalFeedback={renewalFeedback}
        renewalSubmitDisabled={renewalSubmitDisabled}
        delegateRenewalDisabled={!hasAssistantManager || renewalSubmitting}
        onWageChange={setRenewalWage}
        onLengthChange={setRenewalLength}
        onClose={closeRenewalModal}
        onDelegate={() => void handleDelegateRenewal()}
        onSubmit={() => void handleRenewalSubmit()}
      />

      <PlayerProfileTerminationModal
        show={showTerminationModal}
        playerName={player.full_name}
        t={t}
        preview={terminationPreview}
        errorMessage={contractActionError}
        submitting={contractActionSubmitting}
        onCancel={closeTerminationModal}
        onConfirm={() => void handleTerminateContract()}
      />
    </div>
  );
}
