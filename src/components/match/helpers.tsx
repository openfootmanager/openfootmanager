import React from "react";
import type { TFunction } from "i18next";
import { MatchEvent, MatchSnapshot } from "./types";
import {
  Circle, CircleOff, Square, ArrowLeftRight,
  Cross, Play, Pause, Flag, Hand, ArrowUpRight,
  Shield, CornerDownRight, Ruler, AlertTriangle, Zap, CircleDot
} from "lucide-react";

export const EVENT_ICONS: Record<string, { icon: React.ReactNode; color: string; important: boolean }> = {
  Goal:            { icon: <Circle className="w-4 h-4 fill-current" />,          color: "text-accent-400", important: true },
  PenaltyGoal:     { icon: <CircleDot className="w-4 h-4" />,                    color: "text-accent-400", important: true },
  PenaltyMiss:     { icon: <CircleOff className="w-4 h-4" />,                    color: "text-red-400", important: true },
  YellowCard:      { icon: <Square className="w-3.5 h-3.5 fill-yellow-400 text-yellow-400" />, color: "text-yellow-400", important: true },
  RedCard:         { icon: <Square className="w-3.5 h-3.5 fill-red-500 text-red-500" />,       color: "text-red-500", important: true },
  SecondYellow:    { icon: <Square className="w-3.5 h-3.5 fill-red-500 text-red-500" />,       color: "text-red-500", important: true },
  Substitution:    { icon: <ArrowLeftRight className="w-4 h-4" />,               color: "text-blue-400", important: true },
  Injury:          { icon: <Cross className="w-4 h-4" />,                        color: "text-red-400", important: true },
  KickOff:         { icon: <Play className="w-3.5 h-3.5 fill-current" />,        color: "text-gray-400", important: true },
  HalfTime:        { icon: <Pause className="w-3.5 h-3.5" />,                    color: "text-gray-400", important: true },
  SecondHalfStart: { icon: <Play className="w-3.5 h-3.5 fill-current" />,        color: "text-gray-400", important: true },
  FullTime:        { icon: <Flag className="w-4 h-4" />,                         color: "text-gray-400", important: true },
  ShotSaved:       { icon: <Hand className="w-4 h-4" />,                         color: "text-green-400", important: false },
  ShotOffTarget:   { icon: <ArrowUpRight className="w-4 h-4" />,                 color: "text-gray-500", important: false },
  ShotBlocked:     { icon: <Shield className="w-4 h-4" />,                       color: "text-gray-500", important: false },
  Corner:          { icon: <CornerDownRight className="w-4 h-4" />,              color: "text-gray-500", important: false },
  FreeKick:        { icon: <Ruler className="w-4 h-4" />,                        color: "text-gray-500", important: false },
  Foul:            { icon: <AlertTriangle className="w-4 h-4" />,                color: "text-yellow-600", important: false },
  PenaltyAwarded:  { icon: <Zap className="w-4 h-4" />,                          color: "text-accent-400", important: true },
};

const DEFAULT_DISPLAY = { icon: <Circle className="w-3 h-3" />, color: "text-gray-400", important: false };

export function getEventDisplay(evt: MatchEvent) {
  return EVENT_ICONS[evt.event_type] || DEFAULT_DISPLAY;
}

export function getEventLabel(t: TFunction, eventType: string): string {
  const key = `match.eventTypes.${eventType}`;
  const translated = t(key);
  if (translated !== key) {
    return translated;
  }
  return eventType.replace(/([A-Z])/g, " $1").trim();
}

export function getPlayerName(snapshot: MatchSnapshot, playerId: string | null): string {
  if (!playerId) return "";
  for (const p of snapshot.home_team.players) {
    if (p.id === playerId) return p.name;
  }
  for (const p of snapshot.away_team.players) {
    if (p.id === playerId) return p.name;
  }
  // Also check bench players
  if (snapshot.home_bench) {
    for (const p of snapshot.home_bench) {
      if (p.id === playerId) return p.name;
    }
  }
  if (snapshot.away_bench) {
    for (const p of snapshot.away_bench) {
      if (p.id === playerId) return p.name;
    }
  }
  return playerId;
}

export function phaseLabel(t: TFunction, phase: string): string {
  switch (phase) {
    case "PreKickOff": return t("match.phases.preKickOff");
    case "FirstHalf": return t("match.phases.firstHalf");
    case "HalfTime": return t("match.phases.halfTime");
    case "SecondHalf": return t("match.phases.secondHalf");
    case "FullTime": return t("match.phases.fullTime");
    case "ExtraTimeFirstHalf": return t("match.phases.extraTimeFirstHalf");
    case "ExtraTimeHalfTime": return t("match.phases.extraTimeHalfTime");
    case "ExtraTimeSecondHalf": return t("match.phases.extraTimeSecondHalf");
    case "ExtraTimeEnd": return t("match.phases.extraTimeEnd");
    case "PenaltyShootout": return t("match.phases.penaltyShootout");
    case "Finished": return t("match.phases.finished");
    default: return phase;
  }
}

export function calcOvr(attrs: Record<string, number>): number {
  const vals = Object.values(attrs);
  if (vals.length === 0) return 0;
  return Math.round(vals.reduce((a, b) => a + b, 0) / vals.length);
}
