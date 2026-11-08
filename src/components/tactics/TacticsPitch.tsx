import type { JSX } from "react";
import { Star } from "lucide-react";
import { useTranslation } from "react-i18next";

import { getPlayerOvr } from "../../lib/helpers";
import type { FamiliarityTable } from "../../hooks/useFamiliarity";
import type { PositionTable } from "../../lib/positionTable";
import { usePositionTable } from "../../hooks/usePositionGroups";
import type { PlayerData, TeamMatchRolesData } from "../../store/gameStore";
import { Badge, Card, PitchToken, fitLabelKey, type PitchFitTone } from "../ui";
import {
  getSquadTacticalFit,
  translatePositionAbbreviation,
  translatePositionLabel,
} from "../squad/SquadTab.helpers";
import type { TacticsPitchSlot } from "./TacticsTab.helpers";
import type { TacticsPhaseSettings } from "../../store/types";

interface TacticsPitchProps {
  /** The screen's one familiarity table, the one its count and its writers grade with. */
  familiarity: FamiliarityTable;
  /** The table, the familiarity and the squad's answer are all in: tokens state a fit only then. */
  lineupReady: boolean;
  formation: string;
  matchRoles?: TeamMatchRolesData;
  tacticsPhase?: TacticsPhaseSettings;
  /** The player picked in the table, marked here so the two views agree. */
  selectedPlayerId: string | null;
  /** The second pick, marked the same way. */
  comparePlayerId: string | null;
  /** Null until the lineup can be graded: the badge then claims no count. */
  outOfPositionCount: number | null;
  pitchSlots: TacticsPitchSlot[];
}

interface RoleMarker {
  key: keyof TeamMatchRolesData;
  shortLabel: string;
  toneClassName: string;
}

function getFitTone(
  player: PlayerData | null,
  slotPosition: string,
  positionTable: PositionTable,
  familiarity: FamiliarityTable,
  lineupReady: boolean,
): PitchFitTone {
  // The tone waits on what the count waits on, so the pitch never states a verdict in one place
  // and withholds it in another.
  if (!player || !lineupReady) {
    return "empty";
  }

  // No grade (not loaded, or not the club's) is no verdict: the token says nothing of fit.
  switch (getSquadTacticalFit(player, slotPosition, positionTable, familiarity)) {
    case "natural":
      return "exact";
    case "out":
      return "out";
    case "adapted":
      return "adapted";
    default:
      return "empty";
  }
}

function getRoleMarkers(
  matchRoles: TeamMatchRolesData | undefined,
  playerId: string,
): RoleMarker[] {
  if (!matchRoles) {
    return [];
  }

  const markers: RoleMarker[] = [];

  if (matchRoles.captain === playerId) {
    markers.push({
      key: "captain",
      shortLabel: "C",
      toneClassName: "border-accent-500 bg-accent-500 text-white",
    });
  }

  if (matchRoles.vice_captain === playerId) {
    markers.push({
      key: "vice_captain",
      shortLabel: "VC",
      toneClassName: "border-white/60 bg-gray-800/85 text-white",
    });
  }

  if (matchRoles.penalty_taker === playerId) {
    markers.push({
      key: "penalty_taker",
      shortLabel: "PK",
      toneClassName: "border-primary-500 bg-primary-500 text-white",
    });
  }

  if (matchRoles.free_kick_taker === playerId) {
    markers.push({
      key: "free_kick_taker",
      shortLabel: "FK",
      toneClassName: "border-success-600 bg-success-600 text-white",
    });
  }

  if (matchRoles.corner_taker === playerId) {
    markers.push({
      key: "corner_taker",
      shortLabel: "CK",
      toneClassName: "border-orange-500 bg-orange-500 text-white",
    });
  }

  return markers;
}

function getPitchMarkerClassName(isSelected: boolean, isComparing: boolean): string {
  const base =
    "absolute z-20 flex w-[6rem] -translate-x-1/2 -translate-y-1/2 flex-col items-center gap-0.5 rounded-2xl px-1 py-1 text-center";

  if (isSelected) {
    return `${base} bg-accent-500/15 ring-2 ring-accent-300/60 shadow-lg`;
  }

  if (isComparing) {
    return `${base} bg-primary-500/10 ring-2 ring-primary-300/50 shadow-lg`;
  }

  return base;
}

function getSlotTargetClassName(hasPlayer: boolean): string {
  if (hasPlayer) {
    return "absolute z-10 h-[4.5rem] w-[4.5rem] -translate-x-1/2 -translate-y-1/2 rounded-full border border-white/10 bg-transparent";
  }

  return "absolute z-10 h-[4.5rem] w-[4.5rem] -translate-x-1/2 -translate-y-1/2 rounded-full border border-dashed border-white/25 bg-black/10";
}

function getPitchDisplayName(player: PlayerData): string {
  return (player.match_name || player.full_name).toUpperCase();
}

// SVG viewBox is 0 0 100 140; pitch bounds x=[4,96] y=[4,136]; midfield at y=70.
// Team attacks upward (toward y=4). Defensive line sits in the lower half.
function getDefensiveLineY(line: TacticsPhaseSettings["defensive_line"]): number {
  switch (line) {
    case "High":
      return 77;
    case "Low":
      return 105;
    case "VeryLow":
      return 118;
    default:
      return 91; // Medium
  }
}

function getPressingZoneOpacity(intensity: TacticsPhaseSettings["pressing_intensity"]): number {
  switch (intensity) {
    case "Aggressive":
      return 0.13;
    case "Passive":
      return 0;
    default:
      return 0.07; // Medium
  }
}

function getPressingZoneTop(intensity: TacticsPhaseSettings["pressing_intensity"]): number {
  // Aggressive: press from opponent's half; Medium: press from 35m line; Passive: no zone
  switch (intensity) {
    case "Aggressive":
      return 4;
    case "Passive":
      return 70;
    default:
      return 35;
  }
}

function TacticalOverlays({ phase }: { phase: TacticsPhaseSettings }): JSX.Element {
  const lineY = getDefensiveLineY(phase.defensive_line);
  const pressOpacity = getPressingZoneOpacity(phase.pressing_intensity);
  const pressTop = getPressingZoneTop(phase.pressing_intensity);

  return (
    <>
      {/* Pressing zone: shaded band in the opponent's half */}
      {pressOpacity > 0 && (
        <rect
          x="4"
          y={pressTop}
          width="92"
          height={70 - pressTop}
          fill={`rgba(255,220,100,${pressOpacity})`}
          pointerEvents="none"
        />
      )}

      {/* Defensive line: dashed horizontal line */}
      <line
        x1="4"
        y1={lineY}
        x2="96"
        y2={lineY}
        stroke="rgba(255,80,80,0.75)"
        strokeWidth="0.8"
        strokeDasharray="3,2"
        pointerEvents="none"
      />
    </>
  );
}

/** The label key for each match role, as the pitch's markers (C, VC, PK, FK, CK) abbreviate it. */
const ROLE_LABEL_KEYS: Record<keyof TeamMatchRolesData, string> = {
  captain: "preMatch.captain",
  vice_captain: "tactics.viceCaptain",
  penalty_taker: "preMatch.penaltyTaker",
  free_kick_taker: "preMatch.freeKickTaker",
  corner_taker: "preMatch.cornerTaker",
};

export default function TacticsPitch({
  familiarity,
  lineupReady,
  formation,
  matchRoles,
  tacticsPhase,
  selectedPlayerId,
  comparePlayerId,
  outOfPositionCount,
  pitchSlots,
}: TacticsPitchProps): JSX.Element {
  const { t } = useTranslation();
  const positionTable = usePositionTable();
  const title = `${t("preMatch.startingXI")} - ${formation}`;

  // What the drawing says about one slot, in words: the position, the player and, once the lineup
  // can be graded, how he fits it; then the match roles he holds (the drawing's C, VC, PK, FK, CK).
  // An empty slot says so.
  function describeSlot(slot: TacticsPitchSlot): { text: string; roles: string[] } {
    const position = translatePositionLabel(t, slot.position);
    if (!slot.player) return { text: t("tactics.emptySlot", { position }), roles: [] };

    const fitTone = getFitTone(slot.player, slot.position, positionTable, familiarity, lineupReady);
    const fitKey = fitLabelKey(fitTone);
    const condition = Math.round(slot.player.condition);
    const summary = fitKey
      ? t("squad.pitchTokenTooltip", { condition, fit: t(fitKey) })
      : t("squad.pitchTokenConditionOnly", { condition });

    return {
      text: t("tactics.slotSummary", {
        position,
        player: getPitchDisplayName(slot.player),
        summary,
      }),
      roles: getRoleMarkers(matchRoles, slot.player.id).map((marker) =>
        t(ROLE_LABEL_KEYS[marker.key]),
      ),
    };
  }

  return (
    <Card className="overflow-hidden">
      <div className="flex flex-wrap items-center justify-between gap-3 rounded-t-xl border-b border-gray-100 bg-linear-to-r from-navy-700 to-navy-800 px-5 py-4 dark:border-navy-600">
        <h3 className="flex items-center gap-2 text-sm font-heading font-bold uppercase tracking-wide text-white">
          <Star className="h-4 w-4 fill-current text-accent-400" />
          {title}
        </h3>
        <div className="flex flex-wrap items-center justify-end gap-2">
          {outOfPositionCount === null ? null : (
            <Badge variant={outOfPositionCount > 0 ? "danger" : "success"} size="sm">
              {outOfPositionCount} {t("squad.outOfPosition")}
            </Badge>
          )}
          <span className="rounded-full bg-success-500/15 px-2.5 py-1 text-[10px] font-heading font-bold uppercase tracking-widest text-success-300">
            {t("tactics.naturalFit")}
          </span>
          <span className="rounded-full bg-accent-500/15 px-2.5 py-1 text-[10px] font-heading font-bold uppercase tracking-widest text-accent-300">
            {t("tactics.adaptedFit")}
          </span>
          <span className="rounded-full bg-red-500/15 px-2.5 py-1 text-[10px] font-heading font-bold uppercase tracking-widest text-red-300">
            {t("squad.outOfPosition")}
          </span>
        </div>
      </div>

      <div className="p-5 sm:p-6 lg:p-7">
        {/* The pitch only shows the eleven; every change is made in the player table. Its words
            for a screen reader are the list below, so the drawing itself is hidden from them. */}
        <ul aria-label={title} className="sr-only">
          {pitchSlots.map((slot) => {
            const { text, roles } = describeSlot(slot);
            return (
              <li key={slot.index}>
                {text}
                {roles.map((role) => (
                  <span key={role}> {role}</span>
                ))}
              </li>
            );
          })}
        </ul>
        <div
          aria-hidden="true"
          className="relative mx-auto w-full max-w-[36rem] overflow-hidden rounded-[1.5rem] border border-primary-500/20 bg-linear-to-b from-primary-500 to-primary-700 shadow-inner"
        >
          <div className="aspect-[8/10] min-h-[35rem] w-full">
            <svg
              viewBox="0 0 100 140"
              preserveAspectRatio="none"
              className="absolute inset-0 h-full w-full"
              aria-hidden="true"
            >
              <defs>
                <linearGradient id="tactics-pitch-surface" x1="0" x2="0" y1="0" y2="1">
                  <stop offset="0%" stopColor="rgba(63, 172, 99, 0.94)" />
                  <stop offset="100%" stopColor="rgba(31, 109, 61, 0.98)" />
                </linearGradient>
                <pattern
                  id="tactics-pitch-stripes"
                  width="100"
                  height="20"
                  patternUnits="userSpaceOnUse"
                >
                  <rect width="100" height="10" fill="rgba(255,255,255,0.04)" />
                </pattern>
              </defs>
              <rect x="0" y="0" width="100" height="140" fill="url(#tactics-pitch-surface)" />
              <rect x="0" y="0" width="100" height="140" fill="url(#tactics-pitch-stripes)" />
              <rect
                x="4"
                y="4"
                width="92"
                height="132"
                fill="none"
                stroke="rgba(255,255,255,0.55)"
                strokeWidth="0.6"
              />
              <line
                x1="4"
                y1="70"
                x2="96"
                y2="70"
                stroke="rgba(255,255,255,0.55)"
                strokeWidth="0.6"
              />
              <circle
                cx="50"
                cy="70"
                r="11"
                fill="none"
                stroke="rgba(255,255,255,0.55)"
                strokeWidth="0.6"
              />
              <circle cx="50" cy="70" r="0.8" fill="rgba(255,255,255,0.75)" />
              <rect
                x="18"
                y="4"
                width="64"
                height="18"
                fill="none"
                stroke="rgba(255,255,255,0.55)"
                strokeWidth="0.6"
              />
              <rect
                x="31"
                y="4"
                width="38"
                height="8"
                fill="none"
                stroke="rgba(255,255,255,0.55)"
                strokeWidth="0.6"
              />
              <rect
                x="18"
                y="118"
                width="64"
                height="18"
                fill="none"
                stroke="rgba(255,255,255,0.55)"
                strokeWidth="0.6"
              />
              <rect
                x="31"
                y="128"
                width="38"
                height="8"
                fill="none"
                stroke="rgba(255,255,255,0.55)"
                strokeWidth="0.6"
              />
              <path
                d="M 38 22 A 12 12 0 0 0 62 22"
                fill="none"
                stroke="rgba(255,255,255,0.55)"
                strokeWidth="0.6"
              />
              <path
                d="M 38 118 A 12 12 0 0 1 62 118"
                fill="none"
                stroke="rgba(255,255,255,0.55)"
                strokeWidth="0.6"
              />
              {tacticsPhase ? <TacticalOverlays phase={tacticsPhase} /> : null}
            </svg>

            {pitchSlots.map((slot) => {
              const player = slot.player;
              const fitTone = getFitTone(
                player,
                slot.position,
                positionTable,
                familiarity,
                lineupReady,
              );

              return (
                <div
                  key={slot.index}
                  className="absolute"
                  style={{ left: `${slot.x}%`, top: `${slot.y}%` }}
                >
                  <div
                    data-testid={`pitch-slot-${slot.index}`}
                    className={getSlotTargetClassName(!!player)}
                  />

                  {player ? (
                    <div
                      data-testid={`pitch-player-${player.id}`}
                      className={getPitchMarkerClassName(
                        selectedPlayerId === player.id,
                        comparePlayerId === player.id,
                      )}
                    >
                      <PitchToken
                        name={getPitchDisplayName(player)}
                        positionAbbr={translatePositionAbbreviation(t, slot.position)}
                        position={slot.position}
                        ovr={getPlayerOvr(player)}
                        condition={player.condition}
                        fitTone={fitTone}
                        hidePortrait
                        markers={getRoleMarkers(matchRoles, player.id)}
                      />
                    </div>
                  ) : (
                    <div className="absolute z-20 flex w-[4.5rem] -translate-x-1/2 -translate-y-1/2 flex-col items-center text-center">
                      <div className="flex h-11 w-11 items-center justify-center rounded-full border border-dashed border-white/28 bg-black/12 text-[10px] font-heading font-bold uppercase tracking-[0.18em] text-white/70">
                        {translatePositionAbbreviation(t, slot.position)}
                      </div>
                    </div>
                  )}
                </div>
              );
            })}
          </div>
        </div>
      </div>
    </Card>
  );
}
