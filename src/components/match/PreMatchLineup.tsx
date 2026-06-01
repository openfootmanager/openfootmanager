import { useTranslation } from "react-i18next";
import { MatchSnapshot, EnginePlayerData } from "./types";
import { Badge } from "../ui";
import { ArrowUpDown, AlertTriangle, Wand2 } from "lucide-react";
import ContextMenu from "../ContextMenu";
import { translatePositionAbbreviation } from "../squad/SquadTab.helpers";

export const POSITION_KEY_STATS: Record<
  string,
  { label: string; key: string }[]
> = {
  Goalkeeper: [
    { label: "HAN", key: "handling" },
    { label: "REF", key: "reflexes" },
    { label: "AER", key: "aerial" },
  ],
  Defender: [
    { label: "DEF", key: "defending" },
    { label: "TAC", key: "tackling" },
    { label: "STR", key: "strength" },
  ],
  Midfielder: [
    { label: "PAS", key: "passing" },
    { label: "VIS", key: "vision" },
    { label: "STA", key: "stamina" },
  ],
  Forward: [
    { label: "SHO", key: "shooting" },
    { label: "PAC", key: "pace" },
    { label: "DRI", key: "dribbling" },
  ],
};

export function condColor(c: number): string {
  if (c >= 75) return "text-primary-400";
  if (c >= 50) return "text-amber-400";
  return "text-red-400";
}

export function statColor(v: number): string {
  if (v >= 75) return "text-primary-500 dark:text-primary-400 font-bold";
  if (v >= 60) return "text-gray-700 dark:text-gray-200";
  return "text-gray-500 dark:text-gray-400";
}

export function starterOvrColor(ovr: number): string {
  if (ovr >= 70) return "text-primary-600 dark:text-primary-400";
  if (ovr >= 50) return "text-gray-700 dark:text-gray-300";
  return "text-red-600 dark:text-red-400";
}

function hexToRgb(color: string): { red: number; green: number; blue: number } | null {
  const normalized = normalizeHexColor(color);

  if (!normalized) {
    return null;
  }

  const hex = normalized.slice(1);

  return {
    red: Number.parseInt(hex.slice(0, 2), 16),
    green: Number.parseInt(hex.slice(2, 4), 16),
    blue: Number.parseInt(hex.slice(4, 6), 16),
  };
}

function normalizeHexColor(color: string): string | null {
  const normalized = color.trim();
  const hex = normalized.startsWith("#") ? normalized.slice(1) : normalized;

  if (!/^[0-9a-fA-F]{3}$|^[0-9a-fA-F]{6}$/.test(hex)) {
    return null;
  }

  const expanded = hex.length === 3
    ? hex.split("").map((char) => char + char).join("")
    : hex;

  return `#${expanded.toLowerCase()}`;
}

function isLightColor(color: string): boolean {
  const rgb = hexToRgb(color);

  if (!rgb) {
    return false;
  }

  const brightness = (rgb.red * 299 + rgb.green * 587 + rgb.blue * 114) / 1000;
  return brightness >= 205;
}

export function starterBadgeStyle(userColor: string): Record<string, string | number> {
  const normalizedHex = normalizeHexColor(userColor);

  if (!normalizedHex || isLightColor(normalizedHex)) {
    return {
      backgroundColor: "#f8fafc",
      color: "#334155",
      borderColor: "#cbd5e1",
      borderWidth: 1,
      borderStyle: "solid",
    };
  }

  return {
    backgroundColor: `${normalizedHex}30`,
    color: normalizedHex,
    borderColor: `${normalizedHex}55`,
    borderWidth: 1,
    borderStyle: "solid",
  };
}

export function getStatVal(p: EnginePlayerData, key: string): number {
  return (p as unknown as Record<string, number>)[key] ?? 0;
}

export function parseFormationNeeds(formation: string): Record<string, number> {
  const parts = formation
    .split("-")
    .map(Number)
    .filter((n) => !isNaN(n));
  if (parts.length === 3)
    return {
      Goalkeeper: 1,
      Defender: parts[0],
      Midfielder: parts[1],
      Forward: parts[2],
    };
  if (parts.length === 4)
    return {
      Goalkeeper: 1,
      Defender: parts[0],
      Midfielder: parts[1] + parts[2],
      Forward: parts[3],
    };
  return { Goalkeeper: 1, Defender: 4, Midfielder: 4, Forward: 2 };
}

interface PreMatchLineupProps {
  userTeam: MatchSnapshot["home_team"];
  userBench: EnginePlayerData[];
  oppTeam: MatchSnapshot["home_team"];
  userColor: string;
  homeTeamColor: string;
  awayTeamColor: string;
  userSide: "Home" | "Away";
  formationNeeds: Record<string, number>;
  selectedStarterId: string | null;
  isAutoSelecting: boolean;
  onSelectStarter: (id: string | null) => void;
  onSwap: (benchPlayerId: string) => void;
  onAutoSelect: () => void;
}

export default function PreMatchLineup({
  userTeam,
  userBench,
  oppTeam,
  userColor,
  homeTeamColor,
  awayTeamColor,
  userSide,
  formationNeeds,
  selectedStarterId,
  isAutoSelecting,
  onSelectStarter,
  onSwap,
  onAutoSelect,
}: PreMatchLineupProps) {
  const { t } = useTranslation();
  const positions = ["Goalkeeper", "Defender", "Midfielder", "Forward"];

  return (
    <div className="flex flex-col gap-4">
      {/* Formation Balance Bar + Auto-Select */}
      <div className="bg-white dark:bg-navy-800 rounded-xl border border-gray-200 dark:border-navy-700 shadow-sm p-3 flex items-center justify-between transition-colors duration-300">
        <div className="flex items-center gap-4">
          <span className="text-[10px] font-heading uppercase tracking-widest text-gray-700 dark:text-gray-500">
            {t("match.formationFit")}
          </span>
          {(["Goalkeeper", "Defender", "Midfielder", "Forward"] as const).map(
            (pos) => {
              const needed = formationNeeds[pos] || 0;
              const actual = userTeam.players.filter(
                (p) => p.position === pos,
              ).length;
              const ok = actual === needed;
              return (
                <div key={pos} className="flex items-center gap-1">
                  <span className="text-[10px] font-heading uppercase tracking-widest text-gray-600 dark:text-gray-400">
                    {translatePositionAbbreviation(t, pos)}
                  </span>
                  <span
                    className={`text-sm font-heading font-bold tabular-nums ${ok ? "text-primary-700 dark:text-primary-400" : "text-amber-600 dark:text-amber-400"}`}
                  >
                    {actual}/{needed}
                  </span>
                  {!ok && <AlertTriangle className="w-3 h-3 text-amber-600 dark:text-amber-400" />}
                </div>
              );
            },
          )}
        </div>
        <button
          onClick={onAutoSelect}
          disabled={isAutoSelecting}
          className={`flex items-center gap-2 px-4 py-2 rounded-lg text-xs font-heading font-bold uppercase tracking-wider transition-all ${isAutoSelecting
            ? "bg-gray-200 dark:bg-navy-700 text-gray-600 dark:text-gray-400 cursor-wait"
            : "bg-accent-100 text-accent-700 hover:bg-accent-200 dark:bg-accent-500/20 dark:text-accent-300 dark:hover:bg-accent-500/30"
            }`}
        >
          <Wand2 className="w-3.5 h-3.5" />
          {isAutoSelecting ? t("match.selecting") : t("match.autoSelectXI")}
        </button>
      </div>

      <div className="grid grid-cols-2 gap-4">
        {/* Starting XI */}
        <div className="bg-white dark:bg-navy-800 rounded-xl border border-gray-200 dark:border-navy-700 shadow-sm p-4 transition-colors duration-300">
          <div className="flex items-center justify-between mb-3">
            <h3 className="text-xs font-heading font-bold uppercase tracking-widest text-gray-500 dark:text-gray-400">
              {t("match.startingXI")}
            </h3>
            <div className="flex items-center gap-2">
              {selectedStarterId && (
                <button
                  onClick={() => onSelectStarter(null)}
                  className="text-[10px] text-gray-500 hover:text-gray-800 dark:hover:text-gray-300 font-heading uppercase tracking-wider"
                >
                  {t("match.cancel")}
                </button>
              )}
              <Badge variant="primary" size="sm">
                {t("match.nPlayers", { count: userTeam.players.length })}
              </Badge>
            </div>
          </div>
          {selectedStarterId && (
            <p className="text-[10px] text-accent-400 font-heading uppercase tracking-wider mb-2">
              {t("match.swapPrompt")}
            </p>
          )}
          {positions.map((pos) => {
            const players = userTeam.players.filter((p) => p.position === pos);
            if (players.length === 0) return null;
            const needed = formationNeeds[pos] || 0;
            const balanced = players.length === needed;
            const keyStats = POSITION_KEY_STATS[pos] || [];
            return (
              <div key={pos} className="mb-3">
                <div className="flex items-center justify-between mb-1">
                  <div className="flex items-center gap-1.5">
                    <p className="text-[10px] font-heading uppercase tracking-widest text-gray-600">
                      {t(`common.positionGroups.${pos}`)}
                    </p>
                    {!balanced && (
                      <span className="flex items-center gap-0.5">
                        <AlertTriangle className="w-2.5 h-2.5 text-amber-400" />
                        <span className="text-[9px] font-heading text-amber-400">
                          {players.length}/{needed}
                        </span>
                      </span>
                    )}
                  </div>
                  {/* Stat column headers */}
                  <div className="flex items-center gap-0">
                    <span className="text-[8px] font-heading uppercase tracking-widest text-gray-600 dark:text-gray-500 w-7 text-center">
                      OVR
                    </span>
                    {keyStats.map((s) => (
                      <span
                        key={s.label}
                        className="text-[8px] font-heading uppercase tracking-widest text-gray-600 w-7 text-center"
                      >
                        {s.label}
                      </span>
                    ))}
                    <span className="text-[8px] font-heading uppercase tracking-widest text-gray-600 w-8 text-right">
                      FIT
                    </span>
                  </div>
                </div>
                {players.map((p) => {
                  const posOvr = p.ovr;
                  const isSelected = selectedStarterId === p.id;
                  const starterButton = (
                    <button
                      type="button"
                      key={p.id}
                      data-testid={`pre-match-starter-${p.id}`}
                      onClick={() => onSelectStarter(isSelected ? null : p.id)}
                      className={`flex items-center gap-2 py-1.5 px-2 rounded w-full text-left transition-all ${isSelected
                        ? "bg-primary-500/20 ring-1 ring-primary-500/50"
                        : "hover:bg-gray-100 dark:hover:bg-navy-700/50"
                        }`}
                    >
                      <div
                        className="w-7 h-7 rounded-full flex items-center justify-center text-[10px] font-heading font-bold flex-shrink-0"
                        style={starterBadgeStyle(userColor)}
                      >
                        {posOvr}
                      </div>
                      <span className="text-sm text-gray-800 dark:text-gray-200 font-medium flex-1 truncate">
                        {p.name}
                      </span>
                      {isSelected && (
                        <ArrowUpDown className="w-3.5 h-3.5 text-primary-400 flex-shrink-0" />
                      )}
                      <div className="flex items-center gap-0">
                        <span
                          className={`text-[10px] font-heading font-bold tabular-nums w-7 text-center ${starterOvrColor(posOvr)}`}
                        >
                          {posOvr}
                        </span>
                        {keyStats.map((s) => (
                          <span
                            key={s.label}
                            className={`text-[10px] font-heading tabular-nums w-7 text-center ${statColor(getStatVal(p, s.key))}`}
                          >
                            {getStatVal(p, s.key)}
                          </span>
                        ))}
                      </div>
                      <span
                        className={`text-xs tabular-nums w-8 text-right ${condColor(p.condition)}`}
                      >
                        {Math.round(p.condition)}%
                      </span>
                    </button>
                  );

                  return (
                    <ContextMenu
                      items={[
                        {
                          label: isSelected
                            ? t("match.cancel")
                            : t("match.selectForSwap"),
                          onClick: () => onSelectStarter(isSelected ? null : p.id),
                        },
                      ]}
                      key={p.id}
                    >
                      {starterButton}
                    </ContextMenu>
                  );
                })}
              </div>
            );
          })}
        </div>

        {/* Bench */}
        <div className="bg-white dark:bg-navy-800 rounded-xl border border-gray-200 dark:border-navy-700 shadow-sm p-4 transition-colors duration-300">
          <div className="flex items-center justify-between mb-3">
            <h3 className="text-xs font-heading font-bold uppercase tracking-widest text-gray-500 dark:text-gray-400">
              {t("match.substitutes")}
            </h3>
            <Badge variant="neutral" size="sm">
              {t("match.nAvailable", { count: userBench.length })}
            </Badge>
          </div>
          {userBench.length === 0 ? (
            <p className="text-xs text-gray-600 dark:text-gray-500">
              {t("match.noBenchAvailable2")}
            </p>
          ) : (
            <div className="flex flex-col gap-1">
              {/* Bench column header */}
              <div className="flex items-center gap-2 px-2 pb-1">
                <span className="w-7" />
                <span className="flex-1" />
                <span className="text-[8px] font-heading uppercase tracking-widest text-gray-600 dark:text-gray-500 w-8 text-center">
                  POS
                </span>
                <span className="text-[8px] font-heading uppercase tracking-widest text-gray-600 w-[84px] text-center">
                  {t("match.keyStats")}
                </span>
                <span className="text-[8px] font-heading uppercase tracking-widest text-gray-600 w-8 text-right">
                  FIT
                </span>
              </div>
              {userBench.map((bp) => {
                const posOvr = bp.ovr;
                const keyStats = POSITION_KEY_STATS[bp.position] || [];
                const canSwap = Boolean(selectedStarterId);
                const benchButton = (
                  <button
                    type="button"
                    key={bp.id}
                    data-testid={`pre-match-bench-${bp.id}`}
                    disabled={!canSwap}
                    aria-disabled={!canSwap}
                    onClick={() => {
                      if (canSwap) {
                        onSwap(bp.id);
                      }
                    }}
                    className={`flex items-center gap-2 py-1.5 px-2 rounded w-full text-left transition-all ${canSwap
                      ? "hover:bg-primary-500/20 hover:ring-1 hover:ring-primary-500/50 cursor-pointer"
                      : "opacity-60 cursor-not-allowed"
                      }`}
                  >
                    <div className="w-7 h-7 rounded-full bg-gray-200 dark:bg-navy-600 flex items-center justify-center text-[10px] font-heading font-bold text-gray-500 dark:text-gray-400 flex-shrink-0 transition-colors duration-300">
                      {posOvr}
                    </div>
                    <span className="text-sm text-gray-700 dark:text-gray-300 font-medium flex-1 truncate">
                      {bp.name}
                    </span>
                    <Badge variant="neutral" size="sm">
                      {translatePositionAbbreviation(t, bp.position)}
                    </Badge>
                    <div className="flex items-center gap-0">
                      {keyStats.map((s) => (
                        <span
                          key={s.label}
                          className={`text-[10px] font-heading tabular-nums w-7 text-center ${statColor(getStatVal(bp, s.key))}`}
                        >
                          {getStatVal(bp, s.key)}
                        </span>
                      ))}
                    </div>
                    <span
                      className={`text-xs tabular-nums w-8 text-right ${condColor(bp.condition)}`}
                    >
                      {Math.round(bp.condition)}%
                    </span>
                  </button>
                );

                if (!selectedStarterId) {
                  return benchButton;
                }

                return (
                  <ContextMenu
                    items={[
                      {
                        label: t("match.swapWithSelectedStarter"),
                        onClick: () => onSwap(bp.id),
                      },
                    ]}
                    key={bp.id}
                  >
                    {benchButton}
                  </ContextMenu>
                );
              })}
            </div>
          )}

          {/* Opponent Info */}
          <div className="mt-6 pt-4 border-t border-gray-200 dark:border-navy-700">
            <h3 className="text-xs font-heading font-bold uppercase tracking-widest text-gray-500 dark:text-gray-400 mb-3">
              {t("match.opponent")}
            </h3>
            <div className="flex items-center gap-3 mb-2">
              <div
                className="w-10 h-10 rounded-lg flex items-center justify-center font-heading font-bold text-sm"
                style={{
                  backgroundColor:
                    (userSide === "Home" ? awayTeamColor : homeTeamColor) +
                    "30",
                }}
              >
                {oppTeam.name.substring(0, 3).toUpperCase()}
              </div>
              <div>
                <p className="font-heading font-bold text-sm text-gray-800 dark:text-gray-200">
                  {oppTeam.name}
                </p>
                <p className="text-xs text-gray-500 dark:text-gray-400">
                  {oppTeam.formation} · {t(`common.playStyles.${oppTeam.play_style}`, oppTeam.play_style)}
                </p>
              </div>
            </div>
          </div>
        </div>
      </div>
    </div>
  );
}
