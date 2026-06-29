import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { getContractRiskLevel, getPlayerOvr } from "../../lib/helpers";
import { PlayerData, GameStateData } from "../../store/gameStore";
import { ArrowLeft } from "lucide-react";
import { useTranslation } from "react-i18next";
import { resolveTranslatedErrorMessage } from "../../utils/errorMessage";
import { Select } from "../ui";
import { getRoleOptions } from "../../lib/playerRoles";
import { setPlayerRole as setPlayerRoleService } from "../../services/squadService";
import type { PlayerRole } from "../../store/types";
import FreeAgentContractModal from "../transfers/FreeAgentContractModal";
import { useFreeAgentContractFlow } from "../transfers/useFreeAgentContractFlow";
import {
  getPlayerAge,
  getPlayerTeamName,
} from "./PlayerProfile.helpers";
import PlayerProfileAdvancedStatsCard from "./PlayerProfileAdvancedStatsCard";
import { buildPlayerAttributeGroups } from "./PlayerProfile.attributes";
import PlayerProfileAttributesCard from "./PlayerProfileAttributesCard";
import PlayerProfileCareerHistoryCard from "./PlayerProfileCareerHistoryCard";
import PlayerProfileContractCard from "./PlayerProfileContractCard";
import PlayerProfileHeroCard from "./PlayerProfileHeroCard";
import PlayerProfileInjuryBanner from "./PlayerProfileInjuryBanner";
import PlayerProfileLoanStatusBanner from "./PlayerProfileLoanStatusBanner";
import PlayerProfileMovementHistoryCard from "./PlayerProfileMovementHistoryCard";
import PlayerProfileRecentMatchesCard from "./PlayerProfileRecentMatchesCard";
import PlayerProfileRenewalModal from "./PlayerProfileRenewalModal";
import PlayerProfileSeasonStatsCard from "./PlayerProfileSeasonStatsCard";
import {
  getScoutAvailability,
  type PlayerProfileScoutStatus,
} from "./PlayerProfile.scouting";
import { useContractRenewalFlow } from "./useContractRenewalFlow";
import { useContractActionsFlow } from "./useContractActionsFlow";
import PlayerProfileTerminationModal from "./PlayerProfileTerminationModal";
import { usePlayerProfileData } from "./usePlayerProfileData";

interface PlayerProfileProps {
  player: PlayerData;
  gameState: GameStateData;
  isOwnClub: boolean;
  startWithRenewalModal?: boolean;
  startWithTerminationModal?: boolean;
  onClose: () => void;
  onSelectTeam?: (id: string) => void;
  onGameUpdate?: (g: GameStateData) => void;
}

export default function PlayerProfile({
  player,
  gameState,
  isOwnClub,
  startWithRenewalModal = false,
  startWithTerminationModal = false,
  onClose,
  onSelectTeam,
  onGameUpdate,
}: PlayerProfileProps) {
  const { t, i18n } = useTranslation();
  const weeklySuffix = t("finances.perWeekSuffix", "/wk");
  const annualSuffix = t("finances.perYearSuffix", "/yr");
  const primaryPosition = player.natural_position || player.position;
  const footednessLabel = t(
    `common.footedness.${player.footedness || "Right"}`,
  );
  const weakFootValue = player.weak_foot ?? 2;

  const [scoutStatus, setScoutStatus] = useState<PlayerProfileScoutStatus>(
    "idle",
  );
  const [scoutError, setScoutError] = useState<string | null>(null);
  const [hasConsumedInitialRenewalIntent, setHasConsumedInitialRenewalIntent] =
    useState(false);
  const [hasConsumedInitialTerminationIntent, setHasConsumedInitialTerminationIntent] =
    useState(false);
  const ovr = getPlayerOvr(player);
  const age = getPlayerAge(player.date_of_birth);
  const playerTeam = gameState.teams.find((team) => team.id === player.team_id);
  const currentTacticalRole: PlayerRole = playerTeam?.player_roles?.[player.id] ?? "Standard";
  const tacticalRoleOptions = getRoleOptions(primaryPosition, currentTacticalRole);
  const teamName = getPlayerTeamName(
    gameState.teams,
    player.team_id,
    {
      freeAgent: t("common.freeAgent"),
      unknown: t("common.unknown"),
    },
  );
  const contractRiskLevel = getContractRiskLevel(
    player.contract_end,
    gameState.clock.current_date,
  );
  const contractRiskLabel =
    contractRiskLevel === "critical"
      ? t("finances.contractRiskCritical")
      : contractRiskLevel === "warning"
        ? t("finances.contractRiskWarning")
        : t("finances.contractRiskStable");
  const scoutAvailability = getScoutAvailability({
    staff: gameState.staff,
    scoutingAssignments: gameState.scouting_assignments || [],
    youthScoutingAssignments: gameState.youth_scouting_assignments || [],
    managerTeamId: gameState.manager.team_id,
    playerId: player.id,
    scoutStatus,
  });
  const attrGroups = buildPlayerAttributeGroups(player, t);
  const { advancedStats, recentMatches } = usePlayerProfileData({
    player,
    allPlayers: gameState.players,
  });
  const hasLetExpireIntent =
    player.morale_core?.renewal_state?.exit_intent?.kind === "let_expire";
  const isFreeAgent = player.team_id === null && !player.retired;
  const managerTeamId = gameState.manager.team_id;
  const contractOwnerTeamId =
    player.active_loan?.parent_team_id ?? player.team_id ?? null;
  const isContractOwnerClub = Boolean(
    managerTeamId && contractOwnerTeamId === managerTeamId,
  );
  const isManagerOwnedProfile = player.active_loan
    ? isContractOwnerClub
    : isOwnClub || isContractOwnerClub;
  const isManagerLoanClub = Boolean(
    managerTeamId && player.active_loan?.loan_team_id === managerTeamId,
  );
  const isManagerSquadProfile =
    isManagerOwnedProfile || isOwnClub || isManagerLoanClub;
  const hasAssistantManager = managerTeamId
    ? gameState.staff.some(
      (staff) => staff.team_id === managerTeamId && staff.role === "AssistantManager",
    )
    : false;

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
    showRenewalModal,
    renewalWage,
    setRenewalWage,
    renewalLength,
    setRenewalLength,
    renewalIsTerminal,
    renewalCooledOff,
    renewalFeedback,
    renewalProjection,
    isRenewalWageValid,
    renewalViolatesSoftCap,
    renewalSubmitDisabled,
    renewalStatusMessage,
    renewalStatusClassName,
    delegateRenewalDisabled,
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

  const {
    contractActionSubmitting,
    contractActionError,
    terminationPreview,
    showTerminationModal,
    handleMarkLetExpire,
    handleClearLetExpire,
    openTerminationModal,
    handleTerminateContract,
    closeTerminationModal,
  } = useContractActionsFlow({ player, onGameUpdate });

  useEffect(() => {
    setHasConsumedInitialRenewalIntent(false);
    setHasConsumedInitialTerminationIntent(false);
  }, [player.id, startWithRenewalModal, startWithTerminationModal]);

  useEffect(() => {
    if (
      !isManagerOwnedProfile ||
      !startWithRenewalModal ||
      showRenewalModal ||
      hasConsumedInitialRenewalIntent
    ) {
      return;
    }

    setHasConsumedInitialRenewalIntent(true);
    openRenewalModal();
  }, [
    hasConsumedInitialRenewalIntent,
    isManagerOwnedProfile,
    showRenewalModal,
    startWithRenewalModal,
  ]);

  useEffect(() => {
    if (
      !isManagerOwnedProfile ||
      !startWithTerminationModal ||
      showTerminationModal ||
      hasConsumedInitialTerminationIntent
    ) {
      return;
    }

    setHasConsumedInitialTerminationIntent(true);
    void openTerminationModal();
  }, [
    hasConsumedInitialTerminationIntent,
    isManagerOwnedProfile,
    showTerminationModal,
    startWithTerminationModal,
  ]);

  async function handleTacticalRoleChange(role: PlayerRole): Promise<void> {
    if (!onGameUpdate) return;
    try {
      const updated = await setPlayerRoleService(player.id, role);
      onGameUpdate(updated);
    } catch (error) {
      console.error("Failed to set player role:", error);
    }
  }

  return (
    <div>
      <button
        onClick={onClose}
        className="flex items-center gap-2 text-sm text-gray-500 dark:text-gray-400 hover:text-gray-800 dark:hover:text-gray-200 transition-colors mb-4"
      >
        <ArrowLeft className="w-4 h-4" />
        <span className="font-heading font-bold uppercase tracking-wider">
          {t("common.back")}
        </span>
      </button>

      <PlayerProfileHeroCard
        player={player}
            ovr={ovr}
        primaryPosition={primaryPosition}
        age={age}
        teamName={teamName}
        footednessLabel={footednessLabel}
        weakFootValue={weakFootValue}
            annualSuffix={annualSuffix}
        language={i18n.language}
        isOwnClub={isManagerSquadProfile || !onGameUpdate}
        scoutAvailability={scoutAvailability}
        scoutStatus={scoutStatus}
        scoutError={scoutError}
        onScout={() => {
          const availableScout = scoutAvailability.availableScout;
          if (!availableScout || !onGameUpdate) {
            return;
          }

          void (async () => {
            setScoutStatus("sending");
            setScoutError(null);

            try {
              const updated = await invoke<GameStateData>("send_scout", {
                scoutId: availableScout.id,
                playerId: player.id,
              });
              onGameUpdate(updated);
              setScoutStatus("sent");
            } catch (err) {
              setScoutError(resolveTranslatedErrorMessage(err, t));
              setScoutStatus("error");
            }
          })();
        }}
        onSelectTeam={onSelectTeam}
        team={playerTeam}
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

      {/* Injury banner */}
      {player.injury ? (
        <PlayerProfileInjuryBanner injury={player.injury} t={t} />
      ) : null}

      {isOwnClub && onGameUpdate && (
        <div className="mb-4 flex items-center gap-3 rounded-xl border border-gray-200 bg-white px-4 py-3 dark:border-navy-600 dark:bg-navy-800">
          <span className="shrink-0 text-sm font-medium text-gray-600 dark:text-gray-300">
            {t("tactics.playerRoleLabel")}
          </span>
          <Select
            selectSize="sm"
            value={currentTacticalRole}
            onChange={(e) => { void handleTacticalRoleChange(e.target.value as PlayerRole); }}
            aria-label={t("tactics.playerRoleLabel")}
          >
            {tacticalRoleOptions.map((role) => (
              <option key={role} value={role}>
                {t(`tactics.playerRoles.${role}`, role)}
              </option>
            ))}
          </Select>
        </div>
      )}

      {/* Main content grid */}
      <div className="grid grid-cols-1 lg:grid-cols-3 gap-5">
        <PlayerProfileContractCard
          dateOfBirth={player.date_of_birth}
          contractEnd={player.contract_end}
          currentDate={gameState.clock.current_date}
          condition={player.condition}
          morale={player.morale}
              marketValue={player.market_value}
              wage={player.wage}
              annualSuffix={annualSuffix}
          language={i18n.language}
          contractRiskLevel={contractRiskLevel}
          contractRiskLabel={contractRiskLabel}
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

        {contractActionError ? (
          <div className="lg:col-span-3 rounded-lg border border-red-200 bg-red-50 px-4 py-3 text-sm text-red-700 dark:border-red-900/50 dark:bg-red-950/30 dark:text-red-300">
            {contractActionError}
          </div>
        ) : null}

        <PlayerProfileAttributesCard
          attrGroups={attrGroups}
          isOwnClub={isManagerSquadProfile}
          isGk={primaryPosition === "Goalkeeper"}
          title={t("playerProfile.attributes")}
          averageLabel={t("common.average")}
          hiddenTitle={t("playerProfile.attributesHidden")}
          hiddenBody={t("playerProfile.scoutToView")}
          listLabel={t("common.listView")}
          radarLabel={t("common.radarView")}
        />

        <PlayerProfileSeasonStatsCard stats={player.stats} t={t} />

        <PlayerProfileAdvancedStatsCard summary={advancedStats} t={t} />

        <PlayerProfileCareerHistoryCard career={player.career} t={t} />

        <PlayerProfileMovementHistoryCard
          movementHistory={player.movement_history ?? []}
          t={t}
        />

        <PlayerProfileRecentMatchesCard matches={recentMatches} t={t} />
      </div>

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
        weeklySuffix={weeklySuffix}
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
        delegateRenewalDisabled={delegateRenewalDisabled}
        onWageChange={setRenewalWage}
        onLengthChange={setRenewalLength}
        onClose={closeRenewalModal}
        onDelegate={() => void handleDelegateRenewal()}
        onSubmit={() => void handleRenewalSubmit()}
      />

      {showTerminationModal ? (
        <PlayerProfileTerminationModal
          playerName={player.full_name}
          terminationPreview={terminationPreview}
          contractActionError={contractActionError}
          contractActionSubmitting={contractActionSubmitting}
          onCancel={closeTerminationModal}
          onConfirm={() => void handleTerminateContract()}
          t={t}
        />
      ) : null}
    </div>
  );
}
