import type { DragEvent, JSX } from "react";
import {
  Star,
} from "lucide-react";
import { useTranslation } from "react-i18next";

import { getPlayerOvr } from "../../lib/helpers";
import type { PlayerData, TeamMatchRolesData } from "../../store/gameStore";
import ContextMenu from "../ContextMenu";
import { Badge, Card, PitchToken, Select } from "../ui";
import {
  isPlayerExactForSlot,
  isPlayerOutOfPosition,
  translatePositionAbbreviation,
  type DragState,
  type SquadSection,
} from "../squad/SquadTab.helpers";
import type { TacticsPitchSlot } from "./TacticsTab.helpers";
import { buildTacticsPlayerContextMenuItems } from "./TacticsContextMenu.helpers";
import type { KitPattern, PlayerRole, TacticsPhaseSettings } from "../../store/types";
import { getRoleOptions } from "../../lib/playerRoles";

interface TacticsPitchProps {
  benchPlayers?: PlayerData[];
  dragState: DragState | null;
  formation: string;
  matchRoles?: TeamMatchRolesData;
  onRoleChange?: (playerId: string, role: PlayerRole) => void;
  playerRoles?: Record<string, PlayerRole>;
  tacticsPhase?: TacticsPhaseSettings;
  teamKitPattern?: KitPattern;
  teamPrimaryColor?: string;
  teamSecondaryColor?: string;
  comparePlayerId: string | null;
  hoveredSlot: number | null;
  onAssignBestFit?: (playerId: string) => void;
  onAssignMatchRole?: (
    role: keyof TeamMatchRolesData,
    playerId: string,
  ) => void;
  onClearSelection: () => void;
  onDemoteStarter?: (playerId: string) => void;
  onDragStart: (
    event: DragEvent<HTMLElement>,
    playerId: string,
    from: SquadSection,
    slotIndex: number | null,
  ) => void;
  onDragEnd: () => void;
  onLineupPlayerClick: (playerId: string, section: SquadSection) => void;
  onOpenPlayerProfile?: (playerId: string) => void;
  onPromoteBench?: (playerId: string) => void;
  onSlotDragOver: (event: DragEvent<HTMLElement>, slotIndex: number) => void;
  onSlotDragLeave: (slotIndex: number) => void;
  onSlotDrop: (event: DragEvent<HTMLElement>, slotIndex: number) => void;
  outOfPositionCount: number;
  pitchSlots: TacticsPitchSlot[];
  selectedPlayer: PlayerData | null;
  selectedPlayerId: string | null;
}

type FitTone = "exact" | "adapted" | "out" | "empty";

interface RoleMarker {
  key: keyof TeamMatchRolesData;
  shortLabel: string;
  toneClassName: string;
}

function getFitTone(player: PlayerData | null, slotPosition: string): FitTone {
  if (!player) {
    return "empty";
  }

  if (isPlayerExactForSlot(player, slotPosition)) {
    return "exact";
  }

  if (isPlayerOutOfPosition(player, slotPosition)) {
    return "out";
  }

  return "adapted";
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

function getPitchMarkerClassName(options: {
  comparePlayerId: string | null;
  draggedPlayerId: string | null;
  hoveredSlot: number | null;
  player: PlayerData;
  selectedPlayerId: string | null;
  slot: TacticsPitchSlot;
}): string {
  const {
    comparePlayerId,
    draggedPlayerId,
    hoveredSlot,
    player,
    selectedPlayerId,
    slot,
  } = options;
  const isDragged = draggedPlayerId === player.id;
  const isSelected = selectedPlayerId === player.id;
  const isComparing = comparePlayerId === player.id;
  const isHovered = hoveredSlot === slot.index;

  const base =
    "absolute z-20 flex w-[6rem] -translate-x-1/2 -translate-y-1/2 cursor-grab flex-col items-center gap-0.5 rounded-2xl px-1 py-1 text-center transition-all active:cursor-grabbing";

  if (isDragged) {
    return `${base} opacity-60`;
  }

  if (isSelected) {
    return `${base} bg-accent-500/15 ring-2 ring-accent-300/60 shadow-lg`;
  }

  if (isComparing) {
    return `${base} bg-primary-500/10 ring-2 ring-primary-300/50 shadow-lg`;
  }

  if (isHovered) {
    return `${base} bg-primary-500/10 shadow-lg`;
  }

  return `${base} hover:-translate-y-[54%]`;
}


function getSlotTargetClassName(isHovered: boolean, hasPlayer: boolean): string {
  if (isHovered) {
    return "absolute z-10 h-[4.5rem] w-[4.5rem] -translate-x-1/2 -translate-y-1/2 rounded-full border border-primary-300 bg-primary-500/10";
  }

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
    case "High": return 77;
    case "Low": return 105;
    case "VeryLow": return 118;
    default: return 91; // Medium
  }
}

function getPressingZoneOpacity(intensity: TacticsPhaseSettings["pressing_intensity"]): number {
  switch (intensity) {
    case "Aggressive": return 0.13;
    case "Passive": return 0;
    default: return 0.07; // Medium
  }
}

function getPressingZoneTop(intensity: TacticsPhaseSettings["pressing_intensity"]): number {
  // Aggressive: press from opponent's half; Medium: press from 35m line; Passive: no zone
  switch (intensity) {
    case "Aggressive": return 4;
    case "Passive": return 70;
    default: return 35;
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

export default function TacticsPitch({
  dragState,
  formation,
  matchRoles,
  onRoleChange,
  playerRoles,
  tacticsPhase,
  teamKitPattern,
  teamPrimaryColor,
  teamSecondaryColor,
  comparePlayerId,
  hoveredSlot,
  onAssignBestFit,
  onAssignMatchRole,
  onClearSelection,
  onDemoteStarter,
  onDragEnd,
  onDragStart,
  onLineupPlayerClick,
  onOpenPlayerProfile,
  onPromoteBench,
  onSlotDragLeave,
  onSlotDragOver,
  onSlotDrop,
  outOfPositionCount,
  pitchSlots,
  selectedPlayer,
  selectedPlayerId,
}: TacticsPitchProps): JSX.Element {
  const { t } = useTranslation();
  const draggedPlayerId = dragState?.playerId ?? null;

  return (
    <Card className="overflow-hidden">
      <div className="flex flex-wrap items-center justify-between gap-3 rounded-t-xl border-b border-gray-100 bg-linear-to-r from-navy-700 to-navy-800 px-5 py-4 dark:border-navy-600">
        <div>
          <h3 className="flex items-center gap-2 text-sm font-heading font-bold uppercase tracking-wide text-white">
            <Star className="h-4 w-4 fill-current text-accent-400" />
            {t("preMatch.startingXI")} - {formation}
          </h3>
          <p className="mt-0.5 text-xs text-gray-400">
            {t("tactics.pitchInteractionHint")}
          </p>
        </div>
        <div className="flex flex-wrap items-center justify-end gap-2">
          <Badge
            variant={outOfPositionCount > 0 ? "danger" : "success"}
            size="sm"
          >
            {outOfPositionCount} {t("squad.outOfPosition")}
          </Badge>
          <span className="rounded-full bg-success-500/15 px-2.5 py-1 text-[10px] font-heading font-bold uppercase tracking-widest text-success-300">
            {t("tactics.naturalFit")}
          </span>
          <span className="rounded-full bg-accent-500/15 px-2.5 py-1 text-[10px] font-heading font-bold uppercase tracking-widest text-accent-300">
            {t("tactics.adaptedFit")}
          </span>
          <span className="rounded-full bg-red-500/15 px-2.5 py-1 text-[10px] font-heading font-bold uppercase tracking-widest text-red-300">
            {t("squad.outOfPosition")}
          </span>
          <span className="rounded-full bg-white/8 px-2.5 py-1 text-[10px] font-heading font-bold uppercase tracking-widest text-white/70">
            {t("tactics.tableInteractionHint")}
          </span>
          {selectedPlayer ? (
            <button
              type="button"
              onClick={onClearSelection}
              className="text-xs font-heading font-bold uppercase tracking-wider text-accent-400 hover:text-accent-300"
            >
              {t("common.clear")}
            </button>
          ) : null}
        </div>
      </div>

      <div className="p-5 sm:p-6 lg:p-7">
        <div className="relative mx-auto w-full max-w-[36rem] overflow-hidden rounded-[1.5rem] border border-primary-500/20 bg-linear-to-b from-primary-500 to-primary-700 shadow-inner">
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
              const fitTone = getFitTone(player, slot.position);
              const isHovered = hoveredSlot === slot.index;

              return (
                <div
                  key={slot.index}
                  className="absolute"
                  style={{ left: `${slot.x}%`, top: `${slot.y}%` }}
                >
                  <div
                    data-testid={`pitch-slot-${slot.index}`}
                    className={getSlotTargetClassName(isHovered, !!player)}
                    onDragOver={(event) => onSlotDragOver(event, slot.index)}
                    onDragLeave={() => onSlotDragLeave(slot.index)}
                    onDrop={(event) => onSlotDrop(event, slot.index)}
                  />

                  {player ? (
                    (() => {
                      const roleMarkers = getRoleMarkers(matchRoles, player.id);

                      return (
                        <ContextMenu
                          items={buildTacticsPlayerContextMenuItems({
                            isSelected: selectedPlayerId === player.id,
                            matchRoles,
                            onAssignBestFit,
                            onAssignMatchRole,
                            onClearSelection,
                            onDemoteStarter,
                            onOpenProfile: (playerId) => {
                              if (onOpenPlayerProfile) {
                                onOpenPlayerProfile(playerId);
                              } else {
                                onLineupPlayerClick(playerId, "xi");
                              }
                            },
                            onPromoteBench,
                            onTacticalSelect: onLineupPlayerClick,
                            player,
                            section: "xi",
                            selectedPlayerId,
                            t,
                          })}
                        >
                          <div
                            role="button"
                            tabIndex={0}
                            draggable
                            data-testid={`pitch-player-${player.id}`}
                            onClick={() => onLineupPlayerClick(player.id, "xi")}
                            onKeyDown={(e) => {
                              if (e.key === "Enter" || e.key === " ") {
                                e.preventDefault();
                                onLineupPlayerClick(player.id, "xi");
                              }
                            }}
                            onDragOver={(event) => onSlotDragOver(event, slot.index)}
                            onDragLeave={() => onSlotDragLeave(slot.index)}
                            onDrop={(event) => onSlotDrop(event, slot.index)}
                            onDragStart={(event) =>
                              onDragStart(event, player.id, "xi", slot.index)
                            }
                            onDragEnd={onDragEnd}
                            className={getPitchMarkerClassName({
                              comparePlayerId,
                              draggedPlayerId,
                              hoveredSlot,
                              player,
                              selectedPlayerId,
                              slot,
                            })}
                          >
                            <PitchToken
                              name={getPitchDisplayName(player)}
                              positionAbbr={translatePositionAbbreviation(t, slot.position)}
                              position={slot.position}
                              ovr={getPlayerOvr(player)}
                              condition={player.condition}
                              fitTone={fitTone}
                              avatar={player}
                              markers={roleMarkers}
                              jersey={
                                teamSecondaryColor
                                  ? {
                                      primaryColor: teamPrimaryColor ?? "#1a3a6b",
                                      secondaryColor: teamSecondaryColor,
                                      pattern: teamKitPattern ?? "Solid",
                                      number: player.jersey_number,
                                    }
                                  : undefined
                              }
                              jerseyNumber={player.jersey_number}
                            >
                              {/* Role combobox */}
                              {onRoleChange && (
                                <div
                                  draggable={false}
                                  onMouseDown={(e) => e.stopPropagation()}
                                  onKeyDown={(e) => e.stopPropagation()}
                                  className="w-full"
                                >
                                  <Select
                                    selectSize="sm"
                                    variant="ghost"
                                    fullWidth
                                    value={playerRoles?.[player.id] ?? "Standard"}
                                    onChange={(e) => {
                                      onRoleChange(player.id, e.target.value as PlayerRole);
                                    }}
                                  >
                                    {getRoleOptions(
                                      // Roles follow the deployed slot, which is
                                      // what the backend validates against —
                                      // natural-position roles for an
                                      // out-of-position player would be
                                      // rejected and silently revert (#272).
                                      slot.position,
                                      playerRoles?.[player.id] ?? "Standard",
                                    ).map((role) => (
                                      <option key={role} value={role}>
                                        {t(`tactics.playerRoles.${role}`, role)}
                                      </option>
                                    ))}
                                  </Select>
                                </div>
                              )}
                            </PitchToken>
                          </div>
                        </ContextMenu>
                      );
                    })()
                  ) : (
                    <div className="absolute z-20 flex w-[4.5rem] -translate-x-1/2 -translate-y-1/2 flex-col items-center text-center">
                      <div className="flex h-11 w-11 items-center justify-center rounded-full border border-dashed border-white/28 bg-black/12 text-[10px] font-heading font-bold uppercase tracking-[0.18em] text-white/70">
                        {translatePositionAbbreviation(t, slot.position)}
                      </div>
                      <div className="mt-1 max-w-full text-[9px] font-heading font-bold uppercase tracking-[0.16em] text-white/45">
                        {t("squad.dropPlayerHere")}
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
