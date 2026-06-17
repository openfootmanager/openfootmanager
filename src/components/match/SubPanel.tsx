import { useState, type KeyboardEvent } from "react";
import { useTranslation } from "react-i18next";
import { MatchSnapshot } from "./types";
import { getPlayerName } from "./helpers";
import { Badge } from "../ui";
import {
  RefreshCw,
  AlertTriangle,
  UserMinus,
  UserPlus,
  Shield,
  Swords,
  Sparkles,
} from "lucide-react";
import ContextMenu from "../ContextMenu";
import { translatePositionAbbreviation } from "../squad/SquadTab.helpers";
import {
  buildRecommendedSubstitutions,
  getMatchScenario,
  type MatchScenarioId,
  type RecommendationReasonId,
} from "./SubPanel.helpers";

export function SubPanel({
  snapshot,
  side,
  onSubstitute,
  onFormationChange,
  onPlayStyleChange,
  onClose,
}: {
  snapshot: MatchSnapshot;
  side: "Home" | "Away";
  onSubstitute: (offId: string, onId: string) => void;
  onFormationChange: (formation: string) => void;
  onPlayStyleChange: (playStyle: string) => void;
  onClose: () => void;
}) {
  const { t } = useTranslation();
  const [selectedOff, setSelectedOff] = useState<string | null>(null);
  const [selectedBench, setSelectedBench] = useState<string | null>(null);
  const team = side === "Home" ? snapshot.home_team : snapshot.away_team;
  const bench = side === "Home" ? snapshot.home_bench : snapshot.away_bench;
  const subsMade =
    side === "Home" ? snapshot.home_subs_made : snapshot.away_subs_made;
  const subbedOnIds = new Set(
    snapshot.substitutions
      .filter((s) => s.side === side)
      .map((s) => s.player_on_id),
  );
  const subbedOffIds = new Set(
    snapshot.substitutions
      .filter((s) => s.side === side)
      .map((s) => s.player_off_id),
  );
  const availableBench = bench.filter(
    (p) => !subbedOffIds.has(p.id) && !subbedOnIds.has(p.id),
  );
  const selectedPlayer = selectedOff
    ? team.players.find((p) => p.id === selectedOff)
    : null;
  const comparedPlayer = selectedBench
    ? availableBench.find((p) => p.id === selectedBench)
    : null;
  const scenario = getMatchScenario(snapshot, side);
  const recommendations = buildRecommendedSubstitutions(snapshot, side);

  const positions = ["Goalkeeper", "Defender", "Midfielder", "Forward"];

  const condColor = (c: number) =>
    c >= 70 ? "bg-primary-500" : c >= 40 ? "bg-yellow-500" : "bg-red-500";
  const condText = (c: number) =>
    c >= 70 ? "text-primary-400" : c >= 40 ? "text-yellow-400" : "text-red-400";

  const handleClearSelection = () => {
    setSelectedOff(null);
    setSelectedBench(null);
  };

  const handleSelectOffPlayer = (playerId: string) => {
    setSelectedOff((currentSelectedOff) => {
      if (currentSelectedOff === playerId) {
        setSelectedBench(null);
        return null;
      }

      setSelectedBench(null);
      return playerId;
    });
  };

  const handleSelectBenchPlayer = (playerId: string) => {
    if (!selectedOff) {
      return;
    }

    setSelectedBench((currentSelectedBench) => {
      return currentSelectedBench === playerId ? null : playerId;
    });
  };

  const handleConfirmSubstitution = () => {
    if (!selectedOff || !selectedBench) {
      return;
    }

    onSubstitute(selectedOff, selectedBench);
  };

  const handleApplyRecommendation = (offId: string, onId: string) => {
    setSelectedOff(offId);
    setSelectedBench(onId);
  };

  const getScenarioIcon = (scenarioId: MatchScenarioId) => {
    switch (scenarioId) {
      case "protect-lead":
        return <Shield className="h-4 w-4 text-primary-400" />;
      case "chase-goal":
        return <Swords className="h-4 w-4 text-accent-400" />;
      case "find-winner":
        return <Sparkles className="h-4 w-4 text-accent-400" />;
      default:
        return <RefreshCw className="h-4 w-4 text-gray-400" />;
    }
  };

  const getScenarioTitle = (scenarioId: MatchScenarioId): string => {
    return t(`match.subScenario.${scenarioId}.title`);
  };

  const getScenarioDescription = (scenarioId: MatchScenarioId): string => {
    return t(`match.subScenario.${scenarioId}.description`);
  };

  const getRecommendationReasonLabel = (
    reasonId: RecommendationReasonId,
  ): string => {
    return t(`match.subRecommendationReasons.${reasonId}`);
  };

  const getImpactToneClassName = (delta: number): string => {
    if (delta > 0) {
      return "text-success-400";
    }

    if (delta < 0) {
      return "text-red-400";
    }

    return "text-gray-500 dark:text-gray-400";
  };

  const formationOptions = ["4-4-2", "4-3-3", "3-5-2", "4-5-1", "4-2-3-1", "3-4-3"];
  const playStyleOptions = [
    "Balanced",
    "Attacking",
    "Defensive",
    "Possession",
    "Counter",
    "HighPress",
  ];

  const handleInteractiveRowKeyDown = (
    event: KeyboardEvent<HTMLElement>,
    action: () => void,
  ) => {
    if (event.key === "Enter" || event.key === " ") {
      event.preventDefault();
      action();
      return;
    }

    if (event.key === "ContextMenu" || (event.shiftKey && event.key === "F10")) {
      event.preventDefault();
      event.currentTarget.dispatchEvent(
        new MouseEvent("contextmenu", {
          bubbles: true,
          cancelable: true,
        }),
      );
    }
  };

  // Comparison bar component
  const CompareBar = ({
    label,
    valA,
    valB,
  }: {
    label: string;
    valA: number;
    valB: number;
  }) => {
    const diff = valB - valA;
    return (
      <div className="flex items-center gap-2 text-xs py-0.5">
        <span className="w-8 text-right text-gray-500 font-heading">
          {label}
        </span>
        <span className="w-6 text-right tabular-nums text-red-400">{valA}</span>
        <div className="flex-1 h-1.5 bg-navy-600 rounded-full overflow-hidden flex">
          <div className="h-full bg-red-500/60" style={{ width: `${valA}%` }} />
        </div>
        <div className="flex-1 h-1.5 bg-navy-600 rounded-full overflow-hidden flex justify-end">
          <div
            className="h-full bg-green-500/60"
            style={{ width: `${valB}%` }}
          />
        </div>
        <span className="w-6 tabular-nums text-green-400">{valB}</span>
        <span
          className={`w-7 text-right tabular-nums font-heading font-bold ${diff > 0 ? "text-green-400" : diff < 0 ? "text-red-400" : "text-gray-600"}`}
        >
          {diff > 0 ? "+" : ""}
          {diff}
        </span>
      </div>
    );
  };

  return (
    <div
      className="fixed inset-0 z-50 flex items-center justify-center bg-black/70 backdrop-blur-sm"
      onClick={onClose}
    >
      <div
        className="bg-white dark:bg-navy-800 rounded-2xl border border-gray-200 dark:border-navy-600 shadow-2xl w-[1100px] max-h-[90vh] flex flex-col overflow-hidden transition-colors duration-300"
        onClick={(e) => e.stopPropagation()}
      >
        {/* Header */}
        <div className="flex items-center justify-between px-6 py-4 border-b border-gray-200 dark:border-navy-700 bg-linear-to-r from-gray-100 to-white dark:from-navy-700 dark:to-navy-800 transition-colors duration-300">
          <div className="flex items-center gap-3">
            <RefreshCw className="w-5 h-5 text-accent-400" />
            <h3 className="font-heading font-bold text-sm uppercase tracking-widest text-gray-900 dark:text-white">
              {t("match.substitutionsTitle")}
            </h3>
            <Badge
              variant={subsMade >= snapshot.max_subs ? "danger" : "primary"}
              size="sm"
            >
              {t("match.subsUsed", { used: subsMade, max: snapshot.max_subs })}
            </Badge>
          </div>
          <button
            onClick={onClose}
            className="text-gray-500 hover:text-gray-900 p-1.5 rounded-lg hover:bg-gray-100 dark:text-gray-400 dark:hover:text-white dark:hover:bg-navy-600 transition-colors"
          >
            <AlertTriangle className="w-4 h-4 hidden" />
            <span className="text-sm font-heading">✕</span>
          </button>
        </div>

        {subsMade >= snapshot.max_subs ? (
          <div className="flex-1 flex items-center justify-center p-12">
            <div className="flex flex-col items-center gap-3">
              <AlertTriangle className="w-8 h-8 text-yellow-500" />
              <p className="text-sm font-heading font-bold uppercase tracking-wider text-yellow-500">
                {t("match.allSubsUsed")}
              </p>
            </div>
          </div>
        ) : (
          <div className="flex-1 flex overflow-hidden">
            {/* Left: Pitch + On-Field Players */}
            <div className="flex-1 flex flex-col border-r border-gray-200 dark:border-navy-700">
              <div className="px-4 py-3 border-b border-gray-200 dark:border-navy-700 bg-gray-50 dark:bg-navy-800/50 transition-colors duration-300">
                <p className="text-xs font-heading uppercase tracking-widest text-red-400">
                  {selectedOff
                    ? t("match.takingOff", { name: selectedPlayer?.name })
                    : t("match.selectPlayerOff")}
                </p>
              </div>

              <div className="px-4 pt-3">
                <div className="rounded-xl border border-primary-500/20 bg-primary-500/8 px-3 py-3">
                  <div className="flex items-center gap-2">
                    {getScenarioIcon(scenario.id)}
                    <div>
                      <p className="text-xs font-heading font-bold uppercase tracking-widest text-gray-900 dark:text-white">
                        {getScenarioTitle(scenario.id)}
                      </p>
                      <p className="mt-1 text-xs text-gray-600 dark:text-gray-300">
                        {getScenarioDescription(scenario.id)}
                      </p>
                    </div>
                  </div>
                  <div className="mt-3 flex flex-wrap items-center gap-2">
                    <span className="text-[10px] font-heading uppercase tracking-widest text-gray-500 dark:text-gray-400">
                      {t("match.recommendedPlan")}
                    </span>
                    <button
                      type="button"
                      data-testid="recommended-plan-cta"
                      onClick={() => onPlayStyleChange(scenario.recommendedPlayStyle)}
                      className="rounded-full border border-primary-500/25 bg-primary-500/12 px-2.5 py-1 text-[10px] font-heading font-bold uppercase tracking-widest text-primary-500 dark:text-primary-300"
                    >
                      {t(
                        `common.playStyles.${scenario.recommendedPlayStyle}`,
                        scenario.recommendedPlayStyle,
                      )}
                    </button>
                  </div>
                </div>
              </div>

              {/* Mini pitch visualization */}
              <div className="mx-4 mt-3 bg-gradient-to-b from-primary-100 to-primary-50 dark:from-primary-900/30 dark:to-primary-800/10 rounded-xl p-3 relative border border-primary-500/10 min-h-[200px] transition-colors duration-300">
                <div className="absolute inset-x-3 top-1/2 border-t border-gray-300 dark:border-white/5" />
                <div className="absolute left-1/2 top-1/2 -translate-x-1/2 -translate-y-1/2 w-12 h-12 border border-gray-300 dark:border-white/5 rounded-full" />
                {positions.map((pos, rowIdx) => {
                  const players = team.players.filter(
                    (p) =>
                      p.position === pos && !snapshot.sent_off.includes(p.id),
                  );
                  const y = [85, 62, 38, 14][rowIdx];
                  return (
                    <div
                      key={pos}
                      className="absolute left-0 right-0 flex justify-center gap-3"
                      style={{ top: `${y}%`, transform: "translateY(-50%)" }}
                    >
                      {players.map((p) => {
                        const isSelected = selectedOff === p.id;
                        return (
                          <button
                            key={p.id}
                            onClick={() => handleSelectOffPlayer(p.id)}
                            className={`flex flex-col items-center gap-0.5 transition-all cursor-pointer hover:scale-110 ${isSelected ? "scale-110" : ""}`}
                          >
                            <div
                              className={`w-8 h-8 rounded-full flex items-center justify-center text-[9px] font-heading font-bold border-2 transition-all ${isSelected
                                ? "bg-red-500/80 border-red-300 text-white ring-2 ring-red-500/50"
                                : p.condition < 50
                                  ? "bg-yellow-600/70 border-yellow-400 text-white"
                                  : "bg-primary-500/60 border-primary-300/50 text-white"
                                }`}
                            >
                              {Math.round(p.condition)}
                            </div>
                            <span className="text-[9px] text-gray-700 dark:text-white/70 font-medium truncate max-w-[56px]">
                              {p.name.split(" ").pop()}
                            </span>
                          </button>
                        );
                      })}
                    </div>
                  );
                })}
              </div>

              {/* On-field player table */}
              <div className="flex-1 overflow-auto px-4 py-2">
                <table className="w-full text-left">
                  <thead>
                    <tr className="text-[10px] font-heading uppercase tracking-widest text-gray-600 dark:text-gray-500 border-b border-gray-200 dark:border-navy-700">
                      <th className="py-2 pr-2">{t("match.player")}</th>
                      <th className="py-2 w-12 text-center">
                        {t("common.position")}
                      </th>
                      <th className="py-2 w-12 text-center">
                        {t("common.ovr")}
                      </th>
                      <th className="py-2 w-24">{t("match.fitness")}</th>
                    </tr>
                  </thead>
                  <tbody>
                    {team.players
                      .filter((p) => !snapshot.sent_off.includes(p.id))
                      .sort((a, b) => {
                        const posOrd: Record<string, number> = {
                          Goalkeeper: 1,
                          Defender: 2,
                          Midfielder: 3,
                          Forward: 4,
                        };
                        return (
                          (posOrd[a.position] || 99) -
                          (posOrd[b.position] || 99) ||
                          a.name.localeCompare(b.name)
                        );
                      })
                      .map((p) => {
                        const isSelected = selectedOff === p.id;
                        const isSubOn = subbedOnIds.has(p.id);
                        const ovr = p.ovr;
                        const offPlayerRow = (
                          <tr
                            key={p.id}
                            data-testid={`sub-panel-off-${p.id}`}
                            onClick={() => {
                              handleSelectOffPlayer(p.id);
                            }}
                            onKeyDown={(event) => {
                              handleInteractiveRowKeyDown(event, () => {
                                handleSelectOffPlayer(p.id);
                              });
                            }}
                            role="button"
                            tabIndex={0}
                            aria-pressed={isSelected}
                            className={`cursor-pointer transition-colors text-sm ${isSelected
                              ? "bg-red-500/10"
                              : "hover:bg-gray-100 dark:hover:bg-navy-700/50"
                              }`}
                          >
                            <td className="py-2 pr-2">
                              <div className="flex items-center gap-1.5">
                                {isSelected && (
                                  <UserMinus className="w-3.5 h-3.5 text-red-400 shrink-0" />
                                )}
                                {isSubOn && (
                                  <span className="text-green-400 text-[10px]">
                                    ▲
                                  </span>
                                )}
                                <span
                                  className={`font-medium truncate ${isSelected ? "text-red-400" : "text-gray-700 dark:text-gray-300"}`}
                                >
                                  {p.name}
                                </span>
                              </div>
                            </td>
                            <td className="py-2 w-12 text-center">
                              <span className="text-xs font-heading text-gray-500 dark:text-gray-400">
                                {translatePositionAbbreviation(t, p.position)}
                              </span>
                            </td>
                            <td className="py-2 w-12 text-center font-heading font-bold text-gray-500 dark:text-gray-400">
                              {ovr}
                            </td>
                            <td className="py-2 w-24">
                              <div className="flex items-center gap-1.5">
                                <div className="flex-1 h-2 bg-gray-300 dark:bg-navy-600 rounded-full overflow-hidden transition-colors duration-300">
                                  <div
                                    className={`h-full ${condColor(p.condition)} rounded-full`}
                                    style={{ width: `${p.condition}%` }}
                                  />
                                </div>
                                <span
                                  className={`text-xs tabular-nums font-heading w-7 text-right ${condText(p.condition)}`}
                                >
                                  {Math.round(p.condition)}
                                </span>
                              </div>
                            </td>
                          </tr>
                        );

                        return (
                          <ContextMenu
                            items={[
                              {
                                label: isSelected
                                  ? t("common.cancel")
                                  : t("match.selectToTakeOff"),
                                icon: <UserMinus className="w-4 h-4" />,
                                onClick: () => handleSelectOffPlayer(p.id),
                              },
                            ]}
                            key={p.id}
                          >
                            {offPlayerRow}
                          </ContextMenu>
                        );
                      })}
                  </tbody>
                </table>
              </div>
            </div>

            {/* Right: Bench Players + Comparison */}
            <div className="flex-1 flex flex-col">
              <div className="px-4 py-3 border-b border-gray-200 dark:border-navy-700 bg-gray-50 dark:bg-navy-800/50 transition-colors duration-300">
                <p className="text-xs font-heading uppercase tracking-widest text-green-400">
                  {selectedOff
                    ? t("match.selectReplacement")
                    : t("match.benchPlayers")}
                </p>
              </div>

              <div className="px-4 pt-3">
                <div className="rounded-xl border border-gray-200 bg-gray-100 px-3 py-3 dark:border-navy-600 dark:bg-navy-700/40">
                  <div className="flex items-center justify-between gap-3">
                    <p className="text-xs font-heading font-bold uppercase tracking-widest text-gray-700 dark:text-gray-200">
                      {t("match.recommendedChanges")}
                    </p>
                    <Badge variant="accent" size="sm">
                      {recommendations.length}
                    </Badge>
                  </div>
                  {recommendations.length > 0 ? (
                    <div className="mt-3 flex flex-col gap-2">
                      {recommendations.map((recommendation) => {
                        const offPlayer = team.players.find(
                          (player) => player.id === recommendation.offId,
                        );
                        const onPlayer = availableBench.find(
                          (player) => player.id === recommendation.onId,
                        );

                        if (!offPlayer || !onPlayer) {
                          return null;
                        }

                        return (
                          <button
                            key={`${recommendation.offId}-${recommendation.onId}`}
                            type="button"
                            data-testid={`recommended-sub-${recommendation.offId}-${recommendation.onId}`}
                            onClick={() =>
                              handleApplyRecommendation(
                                recommendation.offId,
                                recommendation.onId,
                              )
                            }
                            className="rounded-xl border border-gray-200 bg-white px-3 py-3 text-left transition-colors hover:border-primary-300 hover:bg-primary-50/60 dark:border-navy-600 dark:bg-navy-800 dark:hover:border-primary-400 dark:hover:bg-navy-700"
                          >
                            <div className="flex items-center justify-between gap-3">
                              <div className="min-w-0">
                                <div className="text-sm font-heading font-bold text-gray-900 dark:text-white">
                                  {offPlayer.name} {"->"} {onPlayer.name}
                                </div>
                                <div className="mt-1 text-[11px] uppercase tracking-wider text-gray-500 dark:text-gray-400">
                                  {translatePositionAbbreviation(t, offPlayer.position)} / {translatePositionAbbreviation(t, onPlayer.position)}
                                </div>
                              </div>
                              <div className="text-right text-[11px]">
                                <div className="font-heading font-bold text-success-400">
                                  +{Math.max(0, Math.round(onPlayer.condition - offPlayer.condition))}
                                </div>
                                <div className="text-gray-500 dark:text-gray-400">
                                  {t("match.fitness")}
                                </div>
                              </div>
                            </div>
                            <div className="mt-2 flex flex-wrap gap-1">
                              {recommendation.reasons.map((reason) => (
                                <span
                                  key={reason}
                                  className="rounded-full bg-gray-100 px-2 py-1 text-[10px] font-heading font-bold uppercase tracking-wider text-gray-600 dark:bg-navy-700 dark:text-gray-300"
                                >
                                  {getRecommendationReasonLabel(reason)}
                                </span>
                              ))}
                            </div>
                          </button>
                        );
                      })}
                    </div>
                  ) : (
                    <p className="mt-2 text-xs text-gray-500 dark:text-gray-400">
                      {t("match.noRecommendedChanges")}
                    </p>
                  )}
                </div>

                <div className="mt-3 rounded-xl border border-gray-200 bg-gray-100 px-3 py-3 dark:border-navy-600 dark:bg-navy-700/40">
                  <p className="text-xs font-heading font-bold uppercase tracking-widest text-gray-700 dark:text-gray-200">
                    {t("match.quickTacticalTweaks")}
                  </p>
                  <div className="mt-3">
                    <p className="mb-2 text-[10px] font-heading uppercase tracking-widest text-gray-500 dark:text-gray-400">
                      {t("match.formation")}
                    </p>
                    <div className="flex flex-wrap gap-1">
                      {formationOptions.map((formation) => (
                        <button
                          key={formation}
                          type="button"
                          onClick={() => onFormationChange(formation)}
                          className={`rounded-md px-2 py-1 text-[10px] font-heading font-bold uppercase tracking-wider transition-colors ${
                            team.formation === formation
                              ? "bg-primary-500/20 text-primary-500 ring-1 ring-primary-500/40 dark:text-primary-300"
                              : "bg-white text-gray-600 hover:text-gray-900 dark:bg-navy-800 dark:text-gray-400 dark:hover:text-gray-200"
                          }`}
                        >
                          {formation}
                        </button>
                      ))}
                    </div>
                  </div>
                  <div className="mt-3">
                    <p className="mb-2 text-[10px] font-heading uppercase tracking-widest text-gray-500 dark:text-gray-400">
                      {t("match.playStyle")}
                    </p>
                    <div className="flex flex-wrap gap-1">
                      {playStyleOptions.map((playStyle) => (
                        <button
                          key={playStyle}
                          type="button"
                          onClick={() => onPlayStyleChange(playStyle)}
                          className={`rounded-md px-2 py-1 text-[10px] font-heading font-bold uppercase tracking-wider transition-colors ${
                            team.play_style === playStyle
                              ? "bg-primary-500/20 text-primary-500 ring-1 ring-primary-500/40 dark:text-primary-300"
                              : "bg-white text-gray-600 hover:text-gray-900 dark:bg-navy-800 dark:text-gray-400 dark:hover:text-gray-200"
                          }`}
                        >
                          {t(`common.playStyles.${playStyle}`, playStyle)}
                        </button>
                      ))}
                    </div>
                  </div>
                </div>
              </div>

              {/* Comparison panel */}
              {selectedPlayer && comparedPlayer ? (
                <div className="mx-4 mt-3 p-3 bg-gray-100 dark:bg-navy-700/50 rounded-xl border border-gray-200 dark:border-navy-600 transition-colors duration-300">
                  <div className="flex items-center justify-between mb-2">
                    <div className="flex items-center gap-1.5">
                      <UserMinus className="w-3 h-3 text-red-400" />
                      <span className="text-[10px] text-red-300 font-heading font-bold truncate max-w-[100px]">
                        {selectedPlayer.name}
                      </span>
                    </div>
                    <span className="text-[9px] text-gray-500 dark:text-gray-400 font-heading uppercase">
                      {t("common.vs")}
                    </span>
                    <div className="flex items-center gap-1.5">
                      <span className="text-[10px] text-green-300 font-heading font-bold truncate max-w-[100px]">
                        {comparedPlayer.name}
                      </span>
                      <UserPlus className="w-3 h-3 text-green-400" />
                    </div>
                  </div>
                  <CompareBar
                    label="PAC"
                    valA={selectedPlayer.pace}
                    valB={comparedPlayer.pace}
                  />
                  <CompareBar
                    label="PAS"
                    valA={selectedPlayer.passing}
                    valB={comparedPlayer.passing}
                  />
                  <CompareBar
                    label="SHO"
                    valA={selectedPlayer.shooting}
                    valB={comparedPlayer.shooting}
                  />
                  <CompareBar
                    label="DRI"
                    valA={selectedPlayer.dribbling}
                    valB={comparedPlayer.dribbling}
                  />
                  <CompareBar
                    label="DEF"
                    valA={selectedPlayer.defending}
                    valB={comparedPlayer.defending}
                  />
                  <CompareBar
                    label="TAC"
                    valA={selectedPlayer.tackling}
                    valB={comparedPlayer.tackling}
                  />
                  <CompareBar
                    label="FIT"
                    valA={Math.round(selectedPlayer.condition)}
                    valB={Math.round(comparedPlayer.condition)}
                  />
                  <div className="mt-3 grid grid-cols-3 gap-2">
                    <div className="rounded-lg bg-white px-2 py-2 dark:bg-navy-800">
                      <div className="text-[10px] font-heading uppercase tracking-widest text-gray-500 dark:text-gray-400">
                        {t("common.ovr")}
                      </div>
                      <div
                        className={`mt-1 text-sm font-heading font-bold ${getImpactToneClassName(comparedPlayer.ovr - selectedPlayer.ovr)}`}
                      >
                        {comparedPlayer.ovr - selectedPlayer.ovr > 0 ? "+" : ""}
                        {comparedPlayer.ovr - selectedPlayer.ovr}
                      </div>
                    </div>
                    <div className="rounded-lg bg-white px-2 py-2 dark:bg-navy-800">
                      <div className="text-[10px] font-heading uppercase tracking-widest text-gray-500 dark:text-gray-400">
                        {t("match.fitness")}
                      </div>
                      <div
                        className={`mt-1 text-sm font-heading font-bold ${getImpactToneClassName(
                          Math.round(comparedPlayer.condition - selectedPlayer.condition),
                        )}`}
                      >
                        {comparedPlayer.condition - selectedPlayer.condition > 0
                          ? "+"
                          : ""}
                        {Math.round(comparedPlayer.condition - selectedPlayer.condition)}
                      </div>
                    </div>
                    <div className="rounded-lg bg-white px-2 py-2 dark:bg-navy-800">
                      <div className="text-[10px] font-heading uppercase tracking-widest text-gray-500 dark:text-gray-400">
                        {t("match.roleFit")}
                      </div>
                      <div className="mt-1 text-sm font-heading font-bold text-gray-900 dark:text-white">
                        {comparedPlayer.position === selectedPlayer.position
                          ? t("match.fitExact")
                          : t("match.fitAdjusted")}
                      </div>
                    </div>
                  </div>
                  <div className="mt-3 flex items-center justify-end gap-2">
                    <button
                      type="button"
                      onClick={handleClearSelection}
                      className="rounded-lg border border-gray-300 dark:border-navy-500 px-3 py-2 text-xs font-heading font-bold uppercase tracking-wider text-gray-700 dark:text-gray-300 transition-colors hover:bg-gray-100 dark:hover:bg-navy-600"
                    >
                      {t("common.cancel")}
                    </button>
                    <button
                      type="button"
                      onClick={handleConfirmSubstitution}
                      className="rounded-lg bg-green-500 px-3 py-2 text-xs font-heading font-bold uppercase tracking-wider text-white transition-colors hover:bg-green-400"
                    >
                      {t("match.confirmSubstitution")}
                    </button>
                  </div>
                </div>
              ) : selectedPlayer ? (
                <div className="mx-4 mt-3 p-3 bg-gray-100 dark:bg-navy-700/30 rounded-xl border border-gray-200 dark:border-navy-600/50 text-center transition-colors duration-300">
                  <p className="text-xs text-gray-500 dark:text-gray-400 font-heading uppercase tracking-wider">
                    {t("match.selectBenchToCompare")}
                  </p>
                </div>
              ) : null}

              {/* Bench table */}
              <div className="flex-1 overflow-auto px-4 py-2">
                {availableBench.length === 0 ? (
                  <div className="flex items-center justify-center h-20 text-xs text-gray-600 dark:text-gray-500">
                    {t("match.noBenchAvailable")}
                  </div>
                ) : (
                  <table className="w-full text-left">
                    <thead>
                      <tr className="text-[10px] font-heading uppercase tracking-widest text-gray-600 dark:text-gray-500 border-b border-gray-200 dark:border-navy-700">
                        <th className="py-2 pr-2">{t("match.player")}</th>
                        <th className="py-2 w-12 text-center">
                          {t("common.position")}
                        </th>
                        <th className="py-2 w-12 text-center">
                          {t("common.ovr")}
                        </th>
                        <th className="py-2 w-24">{t("match.fitness")}</th>
                      </tr>
                    </thead>
                    <tbody>
                      {availableBench.map((p) => {
                        const ovr = p.ovr;
                        // Off-position indicator: compare with selected player's position
                        const posMatch = selectedPlayer
                          ? p.position === selectedPlayer.position
                          : true;
                        const benchRow = (
                          <tr
                            key={p.id}
                            data-testid={`sub-panel-bench-${p.id}`}
                            onClick={() => {
                              handleSelectBenchPlayer(p.id);
                            }}
                            onKeyDown={(event) => {
                              handleInteractiveRowKeyDown(event, () => {
                                handleSelectBenchPlayer(p.id);
                              });
                            }}
                            role="button"
                            tabIndex={0}
                            aria-pressed={selectedBench === p.id}
                            aria-disabled={!selectedOff}
                            className={`transition-colors text-sm ${selectedOff
                              ? selectedBench === p.id
                                ? "cursor-pointer bg-green-500/15 ring-1 ring-green-500/30"
                                : "cursor-pointer hover:bg-green-500/10"
                              : "opacity-60"
                              }`}
                          >
                            <td className="py-2 pr-2">
                              <div className="flex items-center gap-1.5">
                                {selectedOff && (
                                  <UserPlus className="w-3.5 h-3.5 text-green-400/50 shrink-0" />
                                )}
                                <span className="font-medium truncate text-gray-700 dark:text-gray-300">
                                  {p.name}
                                </span>
                              </div>
                            </td>
                            <td className="py-2 w-12 text-center">
                              <span
                                className={`text-xs font-heading ${!posMatch && selectedOff ? "text-yellow-400" : "text-gray-500 dark:text-gray-400"}`}
                              >
                                {translatePositionAbbreviation(t, p.position)}
                                {!posMatch && selectedOff && " !"}
                              </span>
                            </td>
                            <td className="py-2 w-12 text-center font-heading font-bold text-gray-500 dark:text-gray-400">
                              {ovr}
                            </td>
                            <td className="py-2 w-24">
                              <div className="flex items-center gap-1.5">
                                <div className="flex-1 h-2 bg-gray-300 dark:bg-navy-600 rounded-full overflow-hidden transition-colors duration-300">
                                  <div
                                    className={`h-full ${condColor(p.condition)} rounded-full`}
                                    style={{ width: `${p.condition}%` }}
                                  />
                                </div>
                                <span
                                  className={`text-xs tabular-nums font-heading w-7 text-right ${condText(p.condition)}`}
                                >
                                  {Math.round(p.condition)}
                                </span>
                              </div>
                            </td>
                          </tr>
                        );

                        return (
                          <ContextMenu
                            items={
                              selectedOff
                                ? [
                                  {
                                    label:
                                      selectedBench === p.id
                                        ? t("match.clearReplacementSelection")
                                        : t("match.selectReplacementMenu"),
                                    icon: <UserPlus className="w-4 h-4" />,
                                    onClick: () => handleSelectBenchPlayer(p.id),
                                  },
                                ]
                                : [
                                  {
                                    label: t("match.selectPlayerToTakeOffFirst"),
                                    icon: <UserPlus className="w-4 h-4" />,
                                    onClick: () => { },
                                    disabled: true,
                                  },
                                ]
                            }
                            key={p.id}
                          >
                            {benchRow}
                          </ContextMenu>
                        );
                      })}
                    </tbody>
                  </table>
                )}
              </div>

              {/* Sub History */}
              {snapshot.substitutions.filter((s) => s.side === side).length >
                0 && (
                  <div className="px-4 py-3 border-t border-gray-200 dark:border-navy-700">
                    <p className="text-[10px] font-heading uppercase tracking-widest text-gray-600 dark:text-gray-500 mb-1.5">
                      {t("match.history")}
                    </p>
                    {snapshot.substitutions
                      .filter((s) => s.side === side)
                      .map((sub, i) => (
                        <div
                          key={i}
                          className="flex items-center gap-1.5 py-0.5 text-[11px]"
                        >
                          <span className="text-gray-600 dark:text-gray-500 tabular-nums w-5 text-right font-heading">
                            {sub.minute}'
                          </span>
                          <span className="text-green-400">▲</span>
                          <span className="text-gray-700 dark:text-gray-300 truncate">
                            {getPlayerName(snapshot, sub.player_on_id)}
                          </span>
                          <span className="text-red-400">▼</span>
                          <span className="text-gray-500 dark:text-gray-400 truncate">
                            {getPlayerName(snapshot, sub.player_off_id)}
                          </span>
                        </div>
                      ))}
                  </div>
                )}
            </div>
          </div>
        )}
      </div>
    </div>
  );
}
