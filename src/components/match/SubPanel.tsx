import { useState, type KeyboardEvent } from "react";
import { useTranslation } from "react-i18next";
import { MatchSnapshot, EnginePlayerData } from "./types";
import { getPlayerName } from "./helpers";
import { Badge } from "../ui";
import { RefreshCw, AlertTriangle, UserMinus, UserPlus } from "lucide-react";
import ContextMenu from "../ContextMenu";
import { translatePositionAbbreviation } from "../squad/SquadTab.helpers";

export function SubPanel({
  snapshot,
  side,
  onSubstitute,
  onClose,
}: {
  snapshot: MatchSnapshot;
  side: "Home" | "Away";
  onSubstitute: (offId: string, onId: string) => void;
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

  const positions = ["Goalkeeper", "Defender", "Midfielder", "Forward"];

  // Parse formation to get expected counts per position
  const parts = team.formation.split("-").map(Number);
  const expectedCounts: Record<string, number> = {
    Goalkeeper: 1,
    Defender: 0,
    Midfielder: 0,
    Forward: 0,
  };
  if (parts.length === 3) {
    expectedCounts.Defender = parts[0];
    expectedCounts.Midfielder = parts[1];
    expectedCounts.Forward = parts[2];
  } else if (parts.length === 4) {
    expectedCounts.Defender = parts[0];
    expectedCounts.Midfielder = parts[1] + parts[2];
    expectedCounts.Forward = parts[3];
  }

  const getOvr = (p: EnginePlayerData) => {
    const vals = [
      p.pace,
      p.stamina,
      p.strength,
      p.passing,
      p.shooting,
      p.tackling,
      p.dribbling,
      p.defending,
      p.positioning,
      p.vision,
      p.decisions,
    ];
    return Math.round(vals.reduce((a, b) => a + b, 0) / vals.length);
  };

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
                        const ovr = getOvr(p);
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
                        const ovr = getOvr(p);
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
