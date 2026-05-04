import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { calcOvr, formatExactMoney, getContractRiskLevel } from "../../lib/helpers";
import { PlayerData, GameStateData } from "../../store/gameStore";
import { ArrowLeft } from "lucide-react";
import { useTranslation } from "react-i18next";
import { resolveBackendText } from "../../utils/backendI18n";
import {
  clearContractExitIntent,
  previewContractTermination,
  setContractExitIntent,
  terminateContractNow,
  type ContractTerminationPreviewData,
} from "../../services/contractService";
import DashboardModalFrame from "../dashboard/DashboardModalFrame";
import { Button } from "../ui";
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
  const primaryPosition = player.natural_position || player.position;
  const footednessLabel = t(
    `common.footedness.${player.footedness || "Right"}`,
    {
      defaultValue: player.footedness || "Right",
    },
  );
  const weakFootValue = player.weak_foot ?? 2;

  if (!player) {
    return null;
  }

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
  const ovr = calcOvr(player, primaryPosition);
  const age = getPlayerAge(player.date_of_birth);
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
    managerTeamId: gameState.manager.team_id,
    playerId: player.id,
    scoutStatus,
  });
  const attrGroups = buildPlayerAttributeGroups(player, t);
  const fallbackAdvancedStats = buildPlayerAdvancedStats(player, gameState.players);
  const advancedStats = advancedStatsOverride ?? fallbackAdvancedStats;
  const hasLetExpireIntent =
    player.morale_core?.renewal_state?.exit_intent?.kind === "let_expire";

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
      !isOwnClub ||
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
    isOwnClub,
    showRenewalModal,
    startWithRenewalModal,
  ]);

  useEffect(() => {
    if (
      !isOwnClub ||
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
    isOwnClub,
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
      setRenewalError(String(error));
      setRenewalCooledOff(false);
    } finally {
      setRenewalSubmitting(false);
    }
  }

  async function handleDelegateRenewal(): Promise<void> {
    if (renewalSubmitting) {
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
      setRenewalError(String(error));
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
      setContractActionError(String(error));
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
      setContractActionError(String(error));
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
      setContractActionError(String(error));
    } finally {
      setContractActionSubmitting(false);
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
      setContractActionError(String(error));
    } finally {
      setContractActionSubmitting(false);
    }
  }

  return (
    <div className="max-w-6xl mx-auto">
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
        weeklySuffix={weeklySuffix}
        language={i18n.language}
        isOwnClub={isOwnClub || !onGameUpdate}
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
              setScoutError(String(err));
              setScoutStatus("error");
            }
          })();
        }}
        onSelectTeam={onSelectTeam}
        t={t}
      />

      {/* Injury banner */}
      {player.injury ? (
        <PlayerProfileInjuryBanner injury={player.injury} t={t} />
      ) : null}

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
          weeklySuffix={weeklySuffix}
          language={i18n.language}
          contractRiskLevel={contractRiskLevel}
          contractRiskLabel={contractRiskLabel}
          isOwnClub={isOwnClub}
          hasLetExpireIntent={hasLetExpireIntent}
          actionSubmitting={contractActionSubmitting}
          onOpenRenewal={openRenewalModal}
          onMarkLetExpire={() => void handleMarkLetExpire()}
          onClearLetExpire={() => void handleClearLetExpire()}
          onOpenTermination={() => void openTerminationModal()}
          t={t}
        />

        {contractActionError ? (
          <div className="lg:col-span-3 rounded-lg border border-red-200 bg-red-50 px-4 py-3 text-sm text-red-700 dark:border-red-900/50 dark:bg-red-950/30 dark:text-red-300">
            {contractActionError}
          </div>
        ) : null}

        <PlayerProfileAttributesCard
          attrGroups={attrGroups}
          isOwnClub={isOwnClub}
          title={t("playerProfile.attributes")}
          averageLabel={t("common.average")}
          hiddenTitle={t("playerProfile.attributesHidden")}
          hiddenBody={t("playerProfile.scoutToView")}
        />

        <PlayerProfileSeasonStatsCard stats={player.stats} t={t} />

        <PlayerProfileAdvancedStatsCard summary={advancedStats} t={t} />

        <PlayerProfileCareerHistoryCard career={player.career} t={t} />

        <PlayerProfileRecentMatchesCard matches={recentMatches} t={t} />
      </div>

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
        renewalSubmitting={renewalSubmitting}
        renewalSubmitDisabled={renewalSubmitDisabled}
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
