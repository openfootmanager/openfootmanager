import { useEffect, useState } from "react";
import { calcOvr, getContractRiskLevel } from "../../lib/helpers";
import { PlayerData, GameStateData } from "../../store/gameStore";
import { ArrowLeft } from "lucide-react";
import { useTranslation } from "react-i18next";
import { resolveBackendText } from "../../utils/backendI18n";
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
import {
  getPlayerMatchHistory,
  getPlayerStatsOverview,
} from "../../services/playerProfileService";
import {
  delegateRenewals,
  type NegotiationFeedbackData as RenewalServiceFeedbackData,
  previewRenewalFinancialImpact,
  proposeRenewal,
} from "../../services/renewalService";
import { sendScout } from "../../services/scoutingService";

interface PlayerProfileProps {
  player: PlayerData;
  gameState: GameStateData;
  isOwnClub: boolean;
  startWithRenewalModal?: boolean;
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

function normaliseRenewalFeedback(
  feedback: RenewalServiceFeedbackData | null | undefined,
): NegotiationFeedbackData | null {
  if (!feedback) {
    return null;
  }

  const mood =
    feedback.mood === "calm" ||
      feedback.mood === "firm" ||
      feedback.mood === "tense" ||
      feedback.mood === "positive" ||
      feedback.mood === "guarded"
      ? feedback.mood
      : "guarded";

  return {
    detail_key: feedback.detail_key ?? null,
    headline_key: feedback.headline_key ?? "common.unknown",
    mood,
    params: feedback.params,
    patience: feedback.patience,
    round: feedback.round,
    tension: feedback.tension,
  };
}

export default function PlayerProfile({
  player,
  gameState,
  isOwnClub,
  startWithRenewalModal = false,
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
  const [advancedStatsOverride, setAdvancedStatsOverride] =
    useState<PlayerAdvancedStatsSummary | null>(null);
  const [recentMatches, setRecentMatches] = useState<PlayerRecentMatchEntry[]>([]);
  const [hasConsumedInitialRenewalIntent, setHasConsumedInitialRenewalIntent] =
    useState(false);
  const ovr = calcOvr(player, primaryPosition);
  const age = getPlayerAge(player.date_of_birth, gameState.clock.current_date);
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
  }, [player.id, startWithRenewalModal]);

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
    if (!showRenewalModal || !isRenewalWageValid) {
      setRenewalProjection(null);
      return;
    }

    let cancelled = false;

    const loadProjection = async (): Promise<void> => {
      try {
        const result = await previewRenewalFinancialImpact(
          player.id,
          renewalOfferedWage,
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
        const result = await getPlayerStatsOverview(player.id);

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
        const result = await getPlayerMatchHistory(player.id, 5);

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
      const result = await proposeRenewal(
        player.id,
        renewalOfferedWage,
        renewalOfferedYears,
      );

      onGameUpdate?.(result.game);
      setRenewalStatus(result.outcome);
      setRenewalSuggestedWage(result.suggested_wage);
      setRenewalSuggestedYears(result.suggested_years);
      setRenewalSessionStatus(result.session_status);
      setRenewalIsTerminal(result.is_terminal);
      setRenewalCooledOff(result.cooled_off ?? false);
      setRenewalFeedback(normaliseRenewalFeedback(result.feedback));

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
      const result = await delegateRenewals({
        playerIds: [player.id],
        maxWageIncreasePct: 35,
        maxContractYears: 3,
      });

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
              const updated = await sendScout(availableScout.id, player.id);
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
          onOpenRenewal={openRenewalModal}
          t={t}
        />

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
    </div>
  );
}
