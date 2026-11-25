import type { JSX } from "react";
import { Star } from "lucide-react";
import { useTranslation } from "react-i18next";

import { getPlayerOvr } from "../../lib/helpers";
import type { FamiliarityTable } from "../../hooks/useFamiliarity";
import type { PositionTable } from "../../lib/positionTable";
import { usePositionTable } from "../../hooks/usePositionGroups";
import type { PlayerData, TeamMatchRolesData } from "../../store/gameStore";
import { Badge, Card, PitchToken, pitchTokenSummary, type PitchFitTone } from "../ui";
import {
  getSquadTacticalFit,
  translatePositionAbbreviation,
  translatePositionLabel,
} from "../squad/SquadTab.helpers";
import type { TacticsPitchSlot } from "./TacticsTab.helpers";
import type { TacticsPhaseSettings } from "../../store/types";
import { MATCH_ROLE_LABEL_KEYS } from "./TacticsRoles.helpers";
import TacticsPitchMarkings from "./TacticsPitchMarkings";

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
    const summary = pitchTokenSummary(t, slot.player.condition, fitTone);

    return {
      text: t("tactics.slotSummary", {
        position,
        player: getPitchDisplayName(slot.player),
        summary,
      }),
      roles: getRoleMarkers(matchRoles, slot.player.id).map((marker) =>
        t(MATCH_ROLE_LABEL_KEYS[marker.key]),
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
            <TacticsPitchMarkings tacticsPhase={tacticsPhase} />

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
