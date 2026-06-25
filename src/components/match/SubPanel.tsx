import { useState, type KeyboardEvent } from "react";
import { useTranslation } from "react-i18next";
import {
  type MatchSnapshot,
  type EnginePlayerData,
  FORMATIONS,
  PLAY_STYLES,
} from "./types";
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
} from "./SubPanel.helpers";

// Parse formation string into player rows from GK (index 0) to FWD (last index).
// Midfielders are distributed across middle rows according to the formation numbers.
function buildFormationPitchRows(
  formation: string,
  players: EnginePlayerData[],
  sentOff: string[],
): EnginePlayerData[][] {
  const active = players.filter((p) => !sentOff.includes(p.id));
  const nums = formation.split("-").map(Number);
  if (!nums.length || nums.some((n) => isNaN(n))) return [active];

  const gks = active.filter((p) => p.position === "Goalkeeper");
  const defs = active.filter((p) => p.position === "Defender");
  const mids = active.filter((p) => p.position === "Midfielder");
  const fwds = active.filter((p) => p.position === "Forward");

  const rows: EnginePlayerData[][] = [gks];
  const n = nums.length;
  let midCursor = 0;

  for (let i = 0; i < n; i++) {
    const count = nums[i];
    if (i === 0) {
      rows.push(defs.slice(0, count));
    } else if (i === n - 1) {
      rows.push(fwds.slice(0, count));
    } else {
      rows.push(mids.slice(midCursor, midCursor + count));
      midCursor += count;
    }
  }

  return rows;
}

// Distribute row y-positions evenly from 85% (GK, bottom) to 12% (FWD, top).
function getRowYPositions(numRows: number): number[] {
  if (numRows === 0) return [];
  if (numRows === 1) return [50];
  const bottom = 85;
  const top = 12;
  const step = (bottom - top) / (numRows - 1);
  return Array.from({ length: numRows }, (_, i) =>
    Math.round(bottom - i * step),
  );
}

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
  const visibleRecommendations = recommendations.flatMap((rec) => {
    const offPlayer = team.players.find((p) => p.id === rec.offId);
    const onPlayer = availableBench.find((p) => p.id === rec.onId);
    if (!offPlayer || !onPlayer) return [];
    return [{ rec, offPlayer, onPlayer }];
  });

  const pitchRows = buildFormationPitchRows(
    team.formation,
    team.players,
    snapshot.sent_off,
  );
  const rowYs = getRowYPositions(pitchRows.length);

  const condColor = (c: number) =>
    c >= 70 ? "bg-primary-500" : c >= 40 ? "bg-yellow-500" : "bg-red-500";
  const condText = (c: number) =>
    c >= 70
      ? "text-primary-400"
      : c >= 40
        ? "text-yellow-400"
        : "text-red-400";

  const getScenarioIcon = (id: MatchScenarioId) => {
    switch (id) {
      case "protect-lead":
        return <Shield className="h-3.5 w-3.5 text-primary-400" />;
      case "chase-goal":
        return <Swords className="h-3.5 w-3.5 text-accent-400" />;
      case "find-winner":
        return <Sparkles className="h-3.5 w-3.5 text-accent-400" />;
      default:
        return <RefreshCw className="h-3.5 w-3.5 text-gray-400" />;
    }
  };

  const getImpactToneClassName = (delta: number) =>
    delta > 0
      ? "text-success-400"
      : delta < 0
        ? "text-red-400"
        : "text-gray-500 dark:text-gray-400";

  const handleClearSelection = () => {
    setSelectedOff(null);
    setSelectedBench(null);
  };

  const handleSelectOffPlayer = (playerId: string) => {
    setSelectedOff((cur) => {
      if (cur === playerId) {
        setSelectedBench(null);
        return null;
      }
      setSelectedBench(null);
      return playerId;
    });
  };

  const handleSelectBenchPlayer = (playerId: string) => {
    if (!selectedOff) return;
    setSelectedBench((cur) => (cur === playerId ? null : playerId));
  };

  const handleConfirmSubstitution = () => {
    if (!selectedOff || !selectedBench) return;
    onSubstitute(selectedOff, selectedBench);
  };

  const handleApplyRecommendation = (offId: string, onId: string) => {
    setSelectedOff(offId);
    setSelectedBench(onId);
  };

  const handleInteractiveRowKeyDown = (
    event: KeyboardEvent<HTMLElement>,
    action: () => void,
  ) => {
    if (event.key === "Enter" || event.key === " ") {
      event.preventDefault();
      action();
      return;
    }
    if (
      event.key === "ContextMenu" ||
      (event.shiftKey && event.key === "F10")
    ) {
      event.preventDefault();
      event.currentTarget.dispatchEvent(
        new MouseEvent("contextmenu", { bubbles: true, cancelable: true }),
      );
    }
  };

  const selectClass =
    "rounded-lg border border-gray-300 dark:border-navy-600 bg-white dark:bg-navy-800 px-2 py-1 text-[11px] font-heading font-bold text-gray-700 dark:text-gray-300 cursor-pointer hover:border-primary-400 transition-colors focus:outline-none focus:ring-1 focus:ring-primary-500";

  return (
    <div
      className="fixed inset-0 z-50 flex items-center justify-center bg-black/70 backdrop-blur-sm p-4"
      onClick={onClose}
    >
      <div
        className="bg-white dark:bg-navy-800 rounded-2xl border border-gray-200 dark:border-navy-600 shadow-2xl w-full max-w-5xl max-h-[90vh] flex flex-col overflow-hidden transition-colors duration-300"
        onClick={(e) => e.stopPropagation()}
      >
        {/* Header */}
        <div className="flex shrink-0 items-center justify-between border-b border-gray-200 bg-linear-to-r from-gray-100 to-white px-5 py-3 dark:border-navy-700 dark:from-navy-700 dark:to-navy-800">
          <div className="flex items-center gap-2.5">
            <RefreshCw className="h-4 w-4 text-accent-400" />
            <h3 className="font-heading text-sm font-bold uppercase tracking-widest text-gray-900 dark:text-white">
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
            className="rounded-lg p-1.5 text-gray-500 transition-colors hover:bg-gray-100 hover:text-gray-900 dark:text-gray-400 dark:hover:bg-navy-600 dark:hover:text-white"
          >
            <span className="font-heading text-sm">✕</span>
          </button>
        </div>

        {subsMade >= snapshot.max_subs ? (
          <div className="flex flex-1 items-center justify-center p-12">
            <div className="flex flex-col items-center gap-3">
              <AlertTriangle className="h-8 w-8 text-yellow-500" />
              <p className="font-heading text-sm font-bold uppercase tracking-wider text-yellow-500">
                {t("match.allSubsUsed")}
              </p>
            </div>
          </div>
        ) : (
          <>
            {/* Tactics strip — scenario, recommendation chips, quick selects */}
            <div className="flex shrink-0 flex-wrap items-center gap-x-3 gap-y-1.5 border-b border-gray-200 bg-gray-50/60 px-4 py-2 dark:border-navy-700 dark:bg-navy-900/30">
              {/* Scenario + apply play style */}
              <div className="flex items-center gap-1.5">
                {getScenarioIcon(scenario.id)}
                <span className="font-heading text-[11px] font-bold uppercase tracking-widest text-gray-800 dark:text-gray-200">
                  {t(`match.subScenario.${scenario.id}.title`)}
                </span>
                <button
                  type="button"
                  data-testid="recommended-plan-cta"
                  onClick={() =>
                    onPlayStyleChange(scenario.recommendedPlayStyle)
                  }
                  className="rounded-full border border-primary-500/25 bg-primary-500/12 px-2 py-0.5 font-heading text-[10px] font-bold uppercase tracking-widest text-primary-500 transition-colors hover:bg-primary-500/20 dark:text-primary-300"
                >
                  {t("match.recommendedPlan")}:{" "}
                  {t(`common.playStyles.${scenario.recommendedPlayStyle}`)}
                </button>
              </div>

              {/* Recommendation chips */}
              {visibleRecommendations.length > 0 && (
                <div className="flex flex-wrap items-center gap-1">
                  {visibleRecommendations.slice(0, 3).map(
                    ({ rec, offPlayer, onPlayer }) => (
                      <button
                        key={`${rec.offId}-${rec.onId}`}
                        type="button"
                        data-testid={`recommended-sub-${rec.offId}-${rec.onId}`}
                        onClick={() =>
                          handleApplyRecommendation(rec.offId, rec.onId)
                        }
                        className="flex items-center gap-1 rounded-full border border-gray-200 bg-white px-2 py-0.5 font-heading text-[10px] font-bold transition-colors hover:border-primary-400 hover:bg-primary-50 dark:border-navy-600 dark:bg-navy-800 dark:hover:bg-navy-700"
                      >
                        <span className="text-red-400">
                          {offPlayer.name.split(" ").pop()}
                        </span>
                        <span className="text-gray-400">→</span>
                        <span className="text-green-400">
                          {onPlayer.name.split(" ").pop()}
                        </span>
                      </button>
                    ),
                  )}
                  {visibleRecommendations.length > 3 && (
                    <span className="font-heading text-[10px] text-gray-400 dark:text-gray-500">
                      +{visibleRecommendations.length - 3}
                    </span>
                  )}
                </div>
              )}

              {/* Quick formation & play style selects */}
              <div className="ml-auto flex items-center gap-2">
                <select
                  value={
                    FORMATIONS.includes(team.formation)
                      ? team.formation
                      : FORMATIONS[0]
                  }
                  onChange={(e) => onFormationChange(e.target.value)}
                  aria-label={t("tactics.formation")}
                  className={selectClass}
                >
                  {FORMATIONS.map((f) => (
                    <option key={f} value={f}>
                      {f}
                    </option>
                  ))}
                </select>
                <select
                  value={team.play_style}
                  onChange={(e) => onPlayStyleChange(e.target.value)}
                  aria-label={t("tactics.playStyle")}
                  className={selectClass}
                >
                  {PLAY_STYLES.map((style) => (
                    <option key={style} value={style}>
                      {t(`common.playStyles.${style}`, style)}
                    </option>
                  ))}
                </select>
              </div>
            </div>

            {/* Main body: two columns */}
            <div className="flex min-h-0 flex-1 overflow-hidden">
              {/* Left: formation pitch + on-field player list */}
              <div className="flex min-w-0 flex-1 flex-col border-r border-gray-200 dark:border-navy-700">
                <div className="shrink-0 border-b border-gray-200 bg-gray-50 px-4 py-2 dark:border-navy-700 dark:bg-navy-800/50">
                  <p className="font-heading text-xs uppercase tracking-widest text-red-400">
                    {selectedOff
                      ? t("match.takingOff", { name: selectedPlayer?.name })
                      : t("match.selectPlayerOff")}
                  </p>
                </div>

                {/* Formation pitch */}
                <div className="relative mx-4 mt-3 h-[200px] shrink-0 overflow-hidden rounded-xl border border-primary-500/10 bg-gradient-to-b from-primary-100 to-primary-50 p-3 transition-colors duration-300 dark:from-primary-900/30 dark:to-primary-800/10">
                  {/* Pitch markings */}
                  <div className="absolute inset-x-3 top-1/2 border-t border-gray-300 dark:border-white/5" />
                  <div className="absolute left-1/2 top-1/2 h-12 w-12 -translate-x-1/2 -translate-y-1/2 rounded-full border border-gray-300 dark:border-white/5" />
                  <div className="absolute inset-x-1/4 bottom-3 h-8 border-x border-b border-gray-300 dark:border-white/5" />
                  {/* Players positioned by formation row */}
                  {pitchRows.map((rowPlayers, rowIdx) => {
                    const y = rowYs[rowIdx] ?? 50;
                    return (
                      <div
                        key={rowIdx}
                        className="absolute left-0 right-0 flex justify-center gap-2 px-2"
                        style={{
                          top: `${y}%`,
                          transform: "translateY(-50%)",
                        }}
                      >
                        {rowPlayers.map((p) => {
                          const isSelected = selectedOff === p.id;
                          const isSubOn = subbedOnIds.has(p.id);
                          return (
                            <button
                              key={p.id}
                              onClick={() => handleSelectOffPlayer(p.id)}
                              className={`flex cursor-pointer flex-col items-center gap-0.5 transition-all hover:scale-110 ${isSelected ? "scale-110" : ""}`}
                            >
                              <div
                                className={`flex h-7 w-7 items-center justify-center rounded-full border-2 font-heading text-[9px] font-bold text-white transition-all ${
                                  isSelected
                                    ? "border-red-300 bg-red-500/80 ring-2 ring-red-500/50"
                                    : p.condition < 50
                                      ? "border-yellow-400 bg-yellow-600/70"
                                      : "border-primary-300/50 bg-primary-500/60"
                                }`}
                              >
                                {isSubOn ? "▲" : Math.round(p.condition)}
                              </div>
                              <span className="max-w-[48px] truncate text-center font-medium text-[8px] text-gray-700 dark:text-white/70">
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
                <div className="min-h-0 flex-1 overflow-auto px-4 py-2">
                  <table className="w-full text-left">
                    <thead>
                      <tr className="border-b border-gray-200 font-heading text-[10px] uppercase tracking-widest text-gray-600 dark:border-navy-700 dark:text-gray-500">
                        <th className="py-2 pr-2">{t("match.player")}</th>
                        <th className="w-12 py-2 text-center">
                          {t("common.position")}
                        </th>
                        <th className="w-12 py-2 text-center">
                          {t("common.ovr")}
                        </th>
                        <th className="w-24 py-2">{t("match.fitness")}</th>
                      </tr>
                    </thead>
                    <tbody>
                      {team.players
                        .filter((p) => !snapshot.sent_off.includes(p.id))
                        .sort((a, b) => {
                          const ord: Record<string, number> = {
                            Goalkeeper: 1,
                            Defender: 2,
                            Midfielder: 3,
                            Forward: 4,
                          };
                          return (
                            (ord[a.position] ?? 99) -
                              (ord[b.position] ?? 99) ||
                            a.name.localeCompare(b.name)
                          );
                        })
                        .map((p) => {
                          const isSelected = selectedOff === p.id;
                          const isSubOn = subbedOnIds.has(p.id);
                          const row = (
                            <tr
                              key={p.id}
                              data-testid={`sub-panel-off-${p.id}`}
                              onClick={() => handleSelectOffPlayer(p.id)}
                              onKeyDown={(e) =>
                                handleInteractiveRowKeyDown(e, () =>
                                  handleSelectOffPlayer(p.id),
                                )
                              }
                              role="button"
                              tabIndex={0}
                              aria-pressed={isSelected}
                              className={`cursor-pointer text-sm transition-colors ${
                                isSelected
                                  ? "bg-red-500/10"
                                  : "hover:bg-gray-100 dark:hover:bg-navy-700/50"
                              }`}
                            >
                              <td className="py-2 pr-2">
                                <div className="flex items-center gap-1.5">
                                  {isSelected && (
                                    <UserMinus className="h-3.5 w-3.5 shrink-0 text-red-400" />
                                  )}
                                  {isSubOn && (
                                    <span className="text-[10px] text-green-400">
                                      ▲
                                    </span>
                                  )}
                                  <span
                                    className={`truncate font-medium ${isSelected ? "text-red-400" : "text-gray-700 dark:text-gray-300"}`}
                                  >
                                    {p.name}
                                  </span>
                                </div>
                              </td>
                              <td className="w-12 py-2 text-center">
                                <span className="font-heading text-xs text-gray-500 dark:text-gray-400">
                                  {translatePositionAbbreviation(
                                    t,
                                    p.position,
                                  )}
                                </span>
                              </td>
                              <td className="w-12 py-2 text-center font-heading font-bold text-gray-500 dark:text-gray-400">
                                {p.ovr}
                              </td>
                              <td className="w-24 py-2">
                                <div className="flex items-center gap-1.5">
                                  <div className="h-2 flex-1 overflow-hidden rounded-full bg-gray-300 dark:bg-navy-600">
                                    <div
                                      className={`h-full rounded-full ${condColor(p.condition)}`}
                                      style={{ width: `${p.condition}%` }}
                                    />
                                  </div>
                                  <span
                                    className={`w-7 text-right font-heading text-xs tabular-nums ${condText(p.condition)}`}
                                  >
                                    {Math.round(p.condition)}
                                  </span>
                                </div>
                              </td>
                            </tr>
                          );
                          return (
                            <ContextMenu
                              key={p.id}
                              items={[
                                {
                                  label: isSelected
                                    ? t("common.cancel")
                                    : t("match.selectToTakeOff"),
                                  icon: <UserMinus className="h-4 w-4" />,
                                  onClick: () => handleSelectOffPlayer(p.id),
                                },
                              ]}
                            >
                              {row}
                            </ContextMenu>
                          );
                        })}
                    </tbody>
                  </table>
                </div>
              </div>

              {/* Right: bench players (full column height) */}
              <div className="flex min-w-0 flex-1 flex-col">
                <div className="shrink-0 border-b border-gray-200 bg-gray-50 px-4 py-2 dark:border-navy-700 dark:bg-navy-800/50">
                  <p className="font-heading text-xs uppercase tracking-widest text-green-400">
                    {selectedOff
                      ? t("match.selectReplacement")
                      : t("match.benchPlayers")}
                  </p>
                </div>

                {availableBench.length === 0 ? (
                  <div className="flex flex-1 items-center justify-center">
                    <p className="text-xs text-gray-600 dark:text-gray-500">
                      {t("match.noBenchAvailable")}
                    </p>
                  </div>
                ) : (
                  <div className="min-h-0 flex-1 overflow-auto px-4 py-2">
                    <table className="w-full text-left">
                      <thead>
                        <tr className="border-b border-gray-200 font-heading text-[10px] uppercase tracking-widest text-gray-600 dark:border-navy-700 dark:text-gray-500">
                          <th className="py-2 pr-2">{t("match.player")}</th>
                          <th className="w-12 py-2 text-center">
                            {t("common.position")}
                          </th>
                          <th className="w-12 py-2 text-center">
                            {t("common.ovr")}
                          </th>
                          <th className="w-24 py-2">{t("match.fitness")}</th>
                        </tr>
                      </thead>
                      <tbody>
                        {availableBench.map((p) => {
                          const posMatch = selectedPlayer
                            ? p.position === selectedPlayer.position
                            : true;
                          const benchRow = (
                            <tr
                              key={p.id}
                              data-testid={`sub-panel-bench-${p.id}`}
                              onClick={() => handleSelectBenchPlayer(p.id)}
                              onKeyDown={(e) =>
                                handleInteractiveRowKeyDown(e, () =>
                                  handleSelectBenchPlayer(p.id),
                                )
                              }
                              role="button"
                              tabIndex={0}
                              aria-pressed={selectedBench === p.id}
                              aria-disabled={!selectedOff}
                              className={`text-sm transition-colors ${
                                selectedOff
                                  ? selectedBench === p.id
                                    ? "cursor-pointer bg-green-500/15 ring-1 ring-green-500/30"
                                    : "cursor-pointer hover:bg-green-500/10"
                                  : "opacity-60"
                              }`}
                            >
                              <td className="py-2 pr-2">
                                <div className="flex items-center gap-1.5">
                                  {selectedOff && (
                                    <UserPlus className="h-3.5 w-3.5 shrink-0 text-green-400/50" />
                                  )}
                                  <span className="truncate font-medium text-gray-700 dark:text-gray-300">
                                    {p.name}
                                  </span>
                                </div>
                              </td>
                              <td className="w-12 py-2 text-center">
                                <span
                                  className={`font-heading text-xs ${!posMatch && selectedOff ? "text-yellow-400" : "text-gray-500 dark:text-gray-400"}`}
                                >
                                  {translatePositionAbbreviation(t, p.position)}
                                  {!posMatch && selectedOff && " !"}
                                </span>
                              </td>
                              <td className="w-12 py-2 text-center font-heading font-bold text-gray-500 dark:text-gray-400">
                                {p.ovr}
                              </td>
                              <td className="w-24 py-2">
                                <div className="flex items-center gap-1.5">
                                  <div className="h-2 flex-1 overflow-hidden rounded-full bg-gray-300 dark:bg-navy-600">
                                    <div
                                      className={`h-full rounded-full ${condColor(p.condition)}`}
                                      style={{ width: `${p.condition}%` }}
                                    />
                                  </div>
                                  <span
                                    className={`w-7 text-right font-heading text-xs tabular-nums ${condText(p.condition)}`}
                                  >
                                    {Math.round(p.condition)}
                                  </span>
                                </div>
                              </td>
                            </tr>
                          );
                          return (
                            <ContextMenu
                              key={p.id}
                              items={
                                selectedOff
                                  ? [
                                      {
                                        label:
                                          selectedBench === p.id
                                            ? t(
                                                "match.clearReplacementSelection",
                                              )
                                            : t("match.selectReplacementMenu"),
                                        icon: <UserPlus className="h-4 w-4" />,
                                        onClick: () =>
                                          handleSelectBenchPlayer(p.id),
                                      },
                                    ]
                                  : [
                                      {
                                        label: t(
                                          "match.selectPlayerToTakeOffFirst",
                                        ),
                                        icon: <UserPlus className="h-4 w-4" />,
                                        onClick: () => {},
                                        disabled: true,
                                      },
                                    ]
                              }
                            >
                              {benchRow}
                            </ContextMenu>
                          );
                        })}
                      </tbody>
                    </table>
                  </div>
                )}

                {/* Sub history */}
                {snapshot.substitutions.filter((s) => s.side === side).length >
                  0 && (
                  <div className="shrink-0 border-t border-gray-200 px-4 py-3 dark:border-navy-700">
                    <p className="mb-1.5 font-heading text-[10px] uppercase tracking-widest text-gray-600 dark:text-gray-500">
                      {t("match.history")}
                    </p>
                    {snapshot.substitutions
                      .filter((s) => s.side === side)
                      .map((sub, i) => (
                        <div
                          key={i}
                          className="flex items-center gap-1.5 py-0.5 text-[11px]"
                        >
                          <span className="w-5 text-right font-heading tabular-nums text-gray-600 dark:text-gray-500">
                            {sub.minute}'
                          </span>
                          <span className="text-green-400">▲</span>
                          <span className="truncate text-gray-700 dark:text-gray-300">
                            {getPlayerName(snapshot, sub.player_on_id)}
                          </span>
                          <span className="text-red-400">▼</span>
                          <span className="truncate text-gray-500 dark:text-gray-400">
                            {getPlayerName(snapshot, sub.player_off_id)}
                          </span>
                        </div>
                      ))}
                  </div>
                )}
              </div>
            </div>

            {/* Sticky footer: comparison summary + confirm / cancel */}
            <div className="shrink-0 border-t border-gray-200 bg-gray-50/60 px-4 py-3 dark:border-navy-700 dark:bg-navy-900/30">
              {selectedPlayer && comparedPlayer ? (
                <div className="flex flex-wrap items-center gap-x-4 gap-y-2">
                  {/* Player names */}
                  <div className="flex items-center gap-2">
                    <div className="flex items-center gap-1.5">
                      <UserMinus className="h-3.5 w-3.5 shrink-0 text-red-400" />
                      <span className="max-w-[130px] truncate font-heading text-sm font-bold text-red-400">
                        {selectedPlayer.name}
                      </span>
                    </div>
                    <span className="text-gray-400">→</span>
                    <div className="flex items-center gap-1.5">
                      <span className="max-w-[130px] truncate font-heading text-sm font-bold text-green-400">
                        {comparedPlayer.name}
                      </span>
                      <UserPlus className="h-3.5 w-3.5 shrink-0 text-green-400" />
                    </div>
                  </div>
                  {/* Key stat deltas */}
                  <div className="flex items-center gap-4 text-xs">
                    <div>
                      <span className="font-heading text-gray-500 dark:text-gray-400">
                        {t("common.ovr")}
                      </span>
                      <span
                        className={`ml-1 font-heading font-bold tabular-nums ${getImpactToneClassName(comparedPlayer.ovr - selectedPlayer.ovr)}`}
                      >
                        {comparedPlayer.ovr - selectedPlayer.ovr > 0
                          ? "+"
                          : ""}
                        {comparedPlayer.ovr - selectedPlayer.ovr}
                      </span>
                    </div>
                    <div>
                      <span className="font-heading text-gray-500 dark:text-gray-400">
                        {t("match.fitness")}
                      </span>
                      <span
                        className={`ml-1 font-heading font-bold tabular-nums ${getImpactToneClassName(Math.round(comparedPlayer.condition - selectedPlayer.condition))}`}
                      >
                        {comparedPlayer.condition - selectedPlayer.condition > 0
                          ? "+"
                          : ""}
                        {Math.round(
                          comparedPlayer.condition - selectedPlayer.condition,
                        )}
                      </span>
                    </div>
                    <span
                      className={`font-heading text-[10px] font-bold uppercase tracking-wide ${
                        comparedPlayer.position === selectedPlayer.position
                          ? "text-green-400"
                          : "text-yellow-400"
                      }`}
                    >
                      {comparedPlayer.position === selectedPlayer.position
                        ? t("match.fitExact")
                        : t("match.fitAdjusted")}
                    </span>
                  </div>
                  {/* Actions */}
                  <div className="ml-auto flex items-center gap-2">
                    <button
                      type="button"
                      onClick={handleClearSelection}
                      className="rounded-lg border border-gray-300 px-3 py-2 font-heading text-xs font-bold uppercase tracking-wider text-gray-700 transition-colors hover:bg-gray-100 dark:border-navy-500 dark:text-gray-300 dark:hover:bg-navy-600"
                    >
                      {t("common.cancel")}
                    </button>
                    <button
                      type="button"
                      onClick={handleConfirmSubstitution}
                      className="rounded-lg bg-green-500 px-3 py-2 font-heading text-xs font-bold uppercase tracking-wider text-white transition-colors hover:bg-green-400"
                    >
                      {t("match.confirmSubstitution")}
                    </button>
                  </div>
                </div>
              ) : selectedPlayer ? (
                <div className="flex items-center gap-2">
                  <UserMinus className="h-3.5 w-3.5 text-red-400" />
                  <span className="font-heading text-sm font-bold text-red-400">
                    {selectedPlayer.name}
                  </span>
                  <span className="text-gray-400">—</span>
                  <span className="font-heading text-xs uppercase tracking-wide text-gray-500 dark:text-gray-400">
                    {t("match.selectBenchToCompare")}
                  </span>
                </div>
              ) : (
                <p className="text-center font-heading text-xs uppercase tracking-widest text-gray-500 dark:text-gray-400">
                  {t("match.selectPlayerOff")}
                </p>
              )}
            </div>
          </>
        )}
      </div>
    </div>
  );
}
