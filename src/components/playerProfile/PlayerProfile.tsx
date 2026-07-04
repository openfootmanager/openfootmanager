import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { formatExactMoney, getContractRiskLevel, getPlayerOvr } from "../../lib/helpers";
import { PlayerData, GameStateData } from "../../store/gameStore";
import { ArrowLeft } from "lucide-react";
import { useTranslation } from "react-i18next";
import { resolveBackendText } from "../../utils/backendI18n";
import { resolveTranslatedErrorMessage } from "../../utils/errorMessage";
import {
  clearContractExitIntent,
  previewContractTermination,
  setContractExitIntent,
  terminateContractNow,
  type ContractTerminationPreviewData,
} from "../../services/contractService";
import DashboardModalFrame from "../dashboard/DashboardModalFrame";
import { Button, Select } from "../ui";
import { getRoleOptions } from "../../lib/playerRoles";
import { getDeployedPosition } from "../squad/SquadTab.helpers";
import { setPlayerRole as setPlayerRoleService } from "../../services/squadService";
import type { PlayerRole } from "../../store/types";
import FreeAgentContractModal from "../transfers/FreeAgentContractModal";
import TransferBidModal from "../transfers/TransferBidModal";
import { useFreeAgentContractFlow } from "../transfers/useFreeAgentContractFlow";
import { useTransferBidFlow } from "../transfers/useTransferBidFlow";
import PlayerProfileActionsMenu from "./PlayerProfileActionsMenu";
import {
  buildPlayerAdvancedStats,
  getPlayerAge,
  getPlayerTeamName,
  type PlayerAdvancedStatsSummary,
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
import PlayerProfileRecentMatchesCard, {
  type PlayerRecentMatchEntry,
} from "./PlayerProfileRecentMatchesCard";
import PlayerProfileRenewalModal from "./PlayerProfileRenewalModal";
import PlayerProfileSeasonStatsCard from "./PlayerProfileSeasonStatsCard";
import {
  type DelegatedRenewalCaseData,
  type DelegatedRenewalResponseData,
  type NegotiationFeedbackData,
  getRenewalStatusClassName,
  getRenewalStatusMessage,
  type RenewalProjectionData,
  type RenewalResponseData,
  type RenewalStatus,
  shouldDisableRenewalSubmit,
} from "./PlayerProfile.renewal";
import {
  getScoutAvailability,
  type PlayerProfileScoutStatus,
} from "./PlayerProfile.scouting";

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

function areAdvancedStatsEqual(
  left: PlayerAdvancedStatsSummary,
  right: PlayerAdvancedStatsSummary,
): boolean {
  return JSON.stringify(left) === JSON.stringify(right);
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
  const [showRenewalModal, setShowRenewalModal] = useState(false);
  const [renewalWage, setRenewalWage] = useState("");
  const [renewalLength, setRenewalLength] = useState("2");
  const [renewalSubmitting, setRenewalSubmitting] = useState(false);
  const [renewalStatus, setRenewalStatus] = useState<RenewalStatus>("idle");
  const [renewalError, setRenewalError] = useState<string | null>(null);
  const [renewalSuggestedWage, setRenewalSuggestedWage] = useState<
    number | null
  >(null);
  const [renewalSuggestedYears, setRenewalSuggestedYears] = useState<
    number | null
  >(null);
  const [renewalSessionStatus, setRenewalSessionStatus] =
    useState<RenewalResponseData["session_status"]>("idle");
  const [renewalIsTerminal, setRenewalIsTerminal] = useState(false);
  const [renewalCooledOff, setRenewalCooledOff] = useState(false);
  const [renewalFeedback, setRenewalFeedback] =
    useState<NegotiationFeedbackData | null>(null);
  const [renewalProjection, setRenewalProjection] =
    useState<RenewalProjectionData["projection"] | null>(null);
  const [contractActionSubmitting, setContractActionSubmitting] = useState(false);
  const [contractActionError, setContractActionError] = useState<string | null>(null);
  const [terminationPreview, setTerminationPreview] =
    useState<ContractTerminationPreviewData | null>(null);
  const [showTerminationModal, setShowTerminationModal] = useState(false);
  const [advancedStatsOverride, setAdvancedStatsOverride] =
    useState<PlayerAdvancedStatsSummary | null>(null);
  const [recentMatches, setRecentMatches] = useState<PlayerRecentMatchEntry[]>([]);
  const [hasConsumedInitialRenewalIntent, setHasConsumedInitialRenewalIntent] =
    useState(false);
  const [hasConsumedInitialTerminationIntent, setHasConsumedInitialTerminationIntent] =
    useState(false);
  const ovr = getPlayerOvr(player);
  const age = getPlayerAge(player.date_of_birth);
  const playerTeam = gameState.teams.find((team) => team.id === player.team_id);
  const currentTacticalRole: PlayerRole = playerTeam?.player_roles?.[player.id] ?? "Standard";
  // Offer roles for the deployed slot when the player is in the starting XI —
  // the backend validates set_player_role against it, so natural-position
  // roles would be rejected for an out-of-position starter (issue #272).
  const roleValidationPosition =
    (playerTeam && getDeployedPosition(playerTeam, player.id)) ||
    primaryPosition;
  const tacticalRoleOptions = getRoleOptions(roleValidationPosition, currentTacticalRole);
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
  const renewalOfferedWage = Number(renewalWage);
  const renewalOfferedYears = Number(renewalLength);
  const isRenewalWageValid =
    Number.isFinite(renewalOfferedWage) && renewalOfferedWage > 0;
  const isRenewalLengthValid =
    Number.isInteger(renewalOfferedYears) && renewalOfferedYears > 0;
  const renewalViolatesSoftCap =
    isRenewalWageValid &&
    renewalProjection !== null &&
    !renewalProjection.policy_allows;
  const renewalSubmitDisabled = shouldDisableRenewalSubmit({
    renewalSubmitting,
    renewalIsTerminal,
    isRenewalWageValid,
    isRenewalLengthValid,
    renewalViolatesSoftCap,
  });
  const renewalStatusMessage = getRenewalStatusMessage(
    {
      renewalSessionStatus,
      renewalStatus,
      renewalSuggestedWage,
      renewalSuggestedYears,
      renewalError,
    },
    t,
  );
  const renewalStatusClassName = getRenewalStatusClassName(renewalStatus);
  const scoutAvailability = getScoutAvailability({
    staff: gameState.staff,
    scoutingAssignments: gameState.scouting_assignments || [],
    youthScoutingAssignments: gameState.youth_scouting_assignments || [],
    managerTeamId: gameState.manager.team_id,
    playerId: player.id,
    scoutStatus,
  });
  const attrGroups = buildPlayerAttributeGroups(player, t);
  const fallbackAdvancedStats = buildPlayerAdvancedStats(player, gameState.players);
  const advancedStats = advancedStatsOverride ?? fallbackAdvancedStats;
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
    bidSubmitDisabled,
    openBidNegotiation,
    closeBidNegotiation,
    handleMakeBid,
  } = useTransferBidFlow({ gameState, onGameUpdate });

  function openRenewalModal(): void {
    setRenewalWage(String(player.wage));
    setRenewalLength("2");
    setRenewalSubmitting(false);
    setRenewalStatus("idle");
    setRenewalError(null);
    setRenewalSuggestedWage(null);
    setRenewalSuggestedYears(null);
    setRenewalSessionStatus("idle");
    setRenewalIsTerminal(false);
    setRenewalCooledOff(false);
    setRenewalFeedback(null);
    setRenewalProjection(null);

    const renewalState = player.morale_core?.renewal_state;
    const blockedUntil = renewalState?.manager_blocked_until;
    const hasActiveManagerBlock =
      renewalState?.status === "Blocked" &&
      (!blockedUntil ||
        blockedUntil.slice(0, 10) >= gameState.clock.current_date.slice(0, 10));
    if (hasActiveManagerBlock) {
      setRenewalSessionStatus("blocked");
      setRenewalIsTerminal(true);
    }

    setShowRenewalModal(true);
  }

  function closeRenewalModal(): void {
    if (renewalSubmitting) {
      return;
    }

    setShowRenewalModal(false);
  }

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

  useEffect(() => {
    if (!showRenewalModal || !isRenewalWageValid) {
      setRenewalProjection(null);
      return;
    }

    let cancelled = false;

    const loadProjection = async (): Promise<void> => {
      try {
        const result = await invoke<RenewalProjectionData>(
          "preview_renewal_financial_impact",
          {
            playerId: player.id,
            weeklyWage: renewalOfferedWage,
          },
        );

        if (!cancelled) {
          setRenewalProjection(result.projection ?? null);
        }
      } catch {
        if (!cancelled) {
          setRenewalProjection(null);
        }
      }
    };

    loadProjection();

    return () => {
      cancelled = true;
    };
  }, [isRenewalWageValid, player.id, renewalOfferedWage, showRenewalModal]);

  useEffect(() => {
    let cancelled = false;

    setAdvancedStatsOverride((current) => (current === null ? current : null));

    const loadAdvancedStats = async (): Promise<void> => {
      try {
        const result = await invoke<PlayerAdvancedStatsSummary>(
          "get_player_stats_overview",
          {
            playerId: player.id,
          },
        );

        if (!cancelled && !areAdvancedStatsEqual(result, fallbackAdvancedStats)) {
          setAdvancedStatsOverride(result);
        }
      } catch {
        if (!cancelled) {
          setAdvancedStatsOverride((current) => (current === null ? current : null));
        }
      }
    };

    void loadAdvancedStats();

    return () => {
      cancelled = true;
    };
  }, [
    player.id,
    player.stats.minutes_played,
    player.stats.shots,
    player.stats.shots_on_target,
    player.stats.passes_completed,
    player.stats.passes_attempted,
    player.stats.tackles_won,
    player.stats.interceptions,
    player.stats.fouls_committed,
  ]);

  useEffect(() => {
    if (player.stats.appearances <= 0) {
      setRecentMatches([]);
      return;
    }

    let cancelled = false;

    const loadRecentMatches = async (): Promise<void> => {
      try {
        const result = await invoke<PlayerRecentMatchEntry[]>(
          "get_player_match_history",
          {
            playerId: player.id,
            limit: 5,
          },
        );

        if (!cancelled) {
          setRecentMatches((current) => {
            if (
              current.length === result.length &&
              current.every(
                (entry, index) => entry.fixture_id === result[index]?.fixture_id,
              )
            ) {
              return current;
            }

            return result;
          });
        }
      } catch {
        if (!cancelled) {
          setRecentMatches((current) =>
            current.length === 0 ? current : [],
          );
        }
      }
    };

    void loadRecentMatches();

    return () => {
      cancelled = true;
    };
  }, [player.id, player.stats.appearances]);

  async function handleRenewalSubmit(): Promise<void> {
    if (renewalSubmitDisabled) {
      return;
    }

    setRenewalSubmitting(true);
    setRenewalStatus("idle");
    setRenewalError(null);
    setRenewalCooledOff(false);

    try {
      const result = await invoke<RenewalResponseData>("propose_renewal", {
        playerId: player.id,
        weeklyWage: renewalOfferedWage,
        contractYears: renewalOfferedYears,
      });

      onGameUpdate?.(result.game);
      setRenewalStatus(result.outcome);
      setRenewalSuggestedWage(result.suggested_wage);
      setRenewalSuggestedYears(result.suggested_years);
      setRenewalSessionStatus(result.session_status);
      setRenewalIsTerminal(result.is_terminal);
      setRenewalCooledOff(result.cooled_off ?? false);
      setRenewalFeedback(result.feedback ?? null);

      if (result.session_status === "blocked") {
        setRenewalStatus("blocked");
      }

      if (result.outcome === "counter_offer") {
        if (result.suggested_wage !== null) {
          setRenewalWage(String(result.suggested_wage));
        }

        if (result.suggested_years !== null) {
          setRenewalLength(String(result.suggested_years));
        }
      }
    } catch (error) {
      setRenewalStatus("error");
      setRenewalError(resolveTranslatedErrorMessage(error, t));
      setRenewalCooledOff(false);
    } finally {
      setRenewalSubmitting(false);
    }
  }

  async function handleDelegateRenewal(): Promise<void> {
    if (renewalSubmitting) {
      return;
    }

    if (!hasAssistantManager) {
      setRenewalStatus("error");
      setRenewalError(
        resolveTranslatedErrorMessage(
          "be.error.contracts.noAssistantManagerAssigned",
          t,
        ),
      );
      setRenewalCooledOff(false);
      return;
    }

    setRenewalSubmitting(true);
    setRenewalError(null);
    setRenewalCooledOff(false);

    try {
      const result = await invoke<DelegatedRenewalResponseData>(
        "delegate_renewals",
        {
          playerIds: [player.id],
          maxWageIncreasePct: 35,
          maxContractYears: 3,
        },
      );

      onGameUpdate?.(result.game);
      const delegatedCase: DelegatedRenewalCaseData | undefined =
        result.report.cases.find(
          (renewalCase) => renewalCase.player_id === player.id,
        );

      if (!delegatedCase) {
        setRenewalStatus("error");
        setRenewalError(t("playerProfile.renewalDelegateMissingReport"));
        return;
      }

      if (delegatedCase.status === "successful") {
        setRenewalStatus("accepted");
        setRenewalSessionStatus("agreed");
        setRenewalIsTerminal(true);
        setRenewalSuggestedWage(null);
        setRenewalSuggestedYears(null);
        setRenewalCooledOff(false);
        setRenewalFeedback(null);
        return;
      }

      if (delegatedCase.status === "stalled") {
        setRenewalStatus("rejected");
        setRenewalSessionStatus("stalled");
        setRenewalIsTerminal(false);
        setRenewalCooledOff(false);
        setRenewalFeedback(null);
        setRenewalError(
          resolveBackendText(
            delegatedCase.note_key,
            delegatedCase.note,
            delegatedCase.note_params,
          ),
        );
        return;
      }

      setRenewalStatus("blocked");
      setRenewalSessionStatus("blocked");
      setRenewalIsTerminal(true);
      setRenewalCooledOff(false);
      setRenewalFeedback(null);
      setRenewalError(
        resolveBackendText(
          delegatedCase.note_key,
          delegatedCase.note,
          delegatedCase.note_params,
        ),
      );
    } catch (error) {
      setRenewalStatus("error");
      setRenewalError(resolveTranslatedErrorMessage(error, t));
      setRenewalCooledOff(false);
    } finally {
      setRenewalSubmitting(false);
    }
  }

  async function handleMarkLetExpire(): Promise<void> {
    if (contractActionSubmitting) {
      return;
    }

    setContractActionSubmitting(true);
    setContractActionError(null);

    try {
      const result = await setContractExitIntent(
        player.id,
        "manager_profile_action",
      );
      onGameUpdate?.(result.game);
    } catch (error) {
      setContractActionError(resolveTranslatedErrorMessage(error, t));
    } finally {
      setContractActionSubmitting(false);
    }
  }

  async function handleClearLetExpire(): Promise<void> {
    if (contractActionSubmitting) {
      return;
    }

    setContractActionSubmitting(true);
    setContractActionError(null);

    try {
      const result = await clearContractExitIntent(player.id);
      onGameUpdate?.(result.game);
    } catch (error) {
      setContractActionError(resolveTranslatedErrorMessage(error, t));
    } finally {
      setContractActionSubmitting(false);
    }
  }

  async function openTerminationModal(): Promise<void> {
    if (contractActionSubmitting) {
      return;
    }

    setContractActionSubmitting(true);
    setContractActionError(null);
    setTerminationPreview(null);
    setShowTerminationModal(true);

    try {
      const result = await previewContractTermination(player.id);
      setTerminationPreview(result.preview);
    } catch (error) {
      setContractActionError(resolveTranslatedErrorMessage(error, t));
    } finally {
      setContractActionSubmitting(false);
    }
  }

  async function handleTacticalRoleChange(role: PlayerRole): Promise<void> {
    if (!onGameUpdate) return;
    try {
      const updated = await setPlayerRoleService(player.id, role);
      onGameUpdate(updated);
    } catch (error) {
      console.error("Failed to set player role:", error);
    }
  }

  async function handleTerminateContract(): Promise<void> {
    if (contractActionSubmitting || !terminationPreview) {
      return;
    }

    setContractActionSubmitting(true);
    setContractActionError(null);

    try {
      const result = await terminateContractNow(player.id);
      onGameUpdate?.(result.game);
      setShowTerminationModal(false);
      setTerminationPreview(null);
    } catch (error) {
      setContractActionError(resolveTranslatedErrorMessage(error, t));
    } finally {
      setContractActionSubmitting(false);
    }
  }

  return (
    <div>
      <div className="mb-4 flex items-center justify-between gap-3">
        <button
          onClick={onClose}
          className="flex items-center gap-2 text-sm text-gray-500 dark:text-gray-400 hover:text-gray-800 dark:hover:text-gray-200 transition-colors"
        >
          <ArrowLeft className="w-4 h-4" />
          <span className="font-heading font-bold uppercase tracking-wider">
            {t("common.back")}
          </span>
        </button>
        {onGameUpdate ? (
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
        ) : null}
      </div>

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
      </div>

      {/* Full-width data cards, stacked for a uniform page */}
      <div className="grid grid-cols-1 gap-5 mt-5">
        <PlayerProfileSeasonStatsCard stats={player.stats} t={t} />

        <PlayerProfileAdvancedStatsCard summary={advancedStats} t={t} />

        <PlayerProfileCareerHistoryCard career={player.career} t={t} />

        <PlayerProfileMovementHistoryCard
          movementHistory={player.movement_history ?? []}
          t={t}
        />

        <PlayerProfileRecentMatchesCard matches={recentMatches} t={t} />
      </div>

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
        delegateRenewalDisabled={!hasAssistantManager || renewalSubmitting}
        onWageChange={setRenewalWage}
        onLengthChange={setRenewalLength}
        onClose={closeRenewalModal}
        onDelegate={() => void handleDelegateRenewal()}
        onSubmit={() => void handleRenewalSubmit()}
      />

      {showTerminationModal ? (
        <DashboardModalFrame maxWidthClassName="max-w-lg">
          <div className="space-y-4">
            <div>
              <h2 className="font-heading text-lg font-bold text-gray-900 dark:text-gray-100">
                {t("playerProfile.terminateContractTitle")}
              </h2>
              <p className="mt-1 text-sm text-gray-600 dark:text-gray-300">
                {t("playerProfile.terminateContractBody", {
                  name: player.full_name,
                })}
              </p>
            </div>

            {terminationPreview ? (
              <div className="rounded-lg border border-gray-200 bg-gray-50 p-4 text-sm dark:border-navy-600 dark:bg-navy-700/60">
                <div className="flex items-center justify-between gap-4">
                  <span className="text-gray-500 dark:text-gray-400">
                    {t("playerProfile.terminationSeverance")}
                  </span>
                  <span className="font-semibold text-gray-900 dark:text-gray-100">
                    {formatExactMoney(terminationPreview.severance_cost)}
                  </span>
                </div>
                <div className="mt-3 flex items-center justify-between gap-4">
                  <span className="text-gray-500 dark:text-gray-400">
                    {t("playerProfile.projectedHealthyPlayers")}
                  </span>
                  <span className="font-semibold text-gray-900 dark:text-gray-100">
                    {terminationPreview.squad_safety.healthy_players}/11
                  </span>
                </div>
                {!terminationPreview.squad_safety.can_field_matchday_squad ? (
                  <p className="mt-3 text-red-600 dark:text-red-300">
                    {t("playerProfile.terminationUnsafe")}
                  </p>
                ) : null}
              </div>
            ) : (
              <p className="text-sm text-gray-500 dark:text-gray-400">
                {t("common.loading")}
              </p>
            )}

            {contractActionError ? (
              <p className="text-sm text-red-600 dark:text-red-300">
                {contractActionError}
              </p>
            ) : null}

            <div className="flex justify-end gap-2">
              <Button
                variant="ghost"
                onClick={() => {
                  setShowTerminationModal(false);
                  setTerminationPreview(null);
                }}
                disabled={contractActionSubmitting}
              >
                {t("common.cancel")}
              </Button>
              <Button
                variant="outline"
                className="text-red-600 hover:text-red-700 dark:text-red-400 dark:hover:text-red-300"
                disabled={
                  contractActionSubmitting ||
                  !terminationPreview?.squad_safety.can_field_matchday_squad
                }
                onClick={() => void handleTerminateContract()}
              >
                {t("playerProfile.confirmTerminateContract")}
              </Button>
            </div>
          </div>
        </DashboardModalFrame>
      ) : null}
    </div>
  );
}
