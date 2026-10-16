import type { JSX } from "react";
import { useMemo, useRef, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { Award, ChevronDown, CircleDot, CornerDownRight, Crown, Footprints } from "lucide-react";
import { useTranslation } from "react-i18next";
import type { GameStateData, PlayerData, TeamMatchRolesData } from "../../store/gameStore";
import type { TacticsPhaseSettings } from "../../store/types";
import SetPieceSelector from "../match/SetPieceSelector";
import { PhaseBlueprintPanel } from "./PhaseBlueprintPanel";
import {
  buildUpdatedMatchRolesForAssignment,
  resolveEffectiveMatchRoles,
} from "./TacticsRoles.helpers";
import { type AutoSetPieces, autoSelectSetPieces } from "../../services/squadService";

interface TacticsRightPanelProps {
  allSquad: PlayerData[];
  /** The backend's takers for this eleven, `null` until it has answered. */
  autoSetPieces: AutoSetPieces | null;
  matchRoles?: TeamMatchRolesData;
  onGameUpdate: (g: GameStateData) => void;
  onTacticsPhaseChange: (patch: Partial<TacticsPhaseSettings>) => void;
  startingPlayers: PlayerData[];
  tacticsPhase?: TacticsPhaseSettings;
}

export default function TacticsRightPanel({
  allSquad,
  autoSetPieces,
  matchRoles,
  onGameUpdate,
  onTacticsPhaseChange,
  startingPlayers,
  tacticsPhase,
}: TacticsRightPanelProps): JSX.Element {
  const { t } = useTranslation();

  const selectorPlayers = useMemo(
    () =>
      startingPlayers.map((player) => ({
        id: player.id,
        name: player.match_name ?? player.full_name,
        position: player.position,
      })),
    [startingPlayers],
  );

  const effectiveRoles = useMemo(
    () => resolveEffectiveMatchRoles(startingPlayers, matchRoles, autoSetPieces),
    [matchRoles, startingPlayers, autoSetPieces],
  );

  async function persistMatchRoles(nextRoles: TeamMatchRolesData): Promise<void> {
    try {
      const updated = await invoke<GameStateData>("set_team_match_roles", {
        matchRoles: nextRoles,
      });
      onGameUpdate(updated);
    } catch (error) {
      console.error("Failed to set team match roles:", error);
    }
  }

  async function handleRoleChange(role: keyof TeamMatchRolesData, playerId: string): Promise<void> {
    await persistMatchRoles(
      buildUpdatedMatchRolesForAssignment(effectiveRoles, role, playerId, autoSetPieces),
    );
  }

  // What the eleven and the saved roles are right now, read when an answer lands: an ask can
  // outlive the screen it was made from, and its answer must not overwrite what the manager did
  // meanwhile (the same guard `useEntityEditor.commitField` keeps for an asset pick).
  const rolesShown = JSON.stringify([startingPlayers.map((player) => player.id), matchRoles]);
  const latestRolesShown = useRef(rolesShown);
  latestRolesShown.current = rolesShown;

  // Asks the backend now rather than saving what the display last showed: an answer still on its
  // way, or one a day old, would save empty or stale takers. A failed ask saves nothing, and
  // pressing the button again is the retry.
  async function handleAutoSelectAssignments(): Promise<void> {
    try {
      const picks = await autoSelectSetPieces(startingPlayers.map((player) => player.id));
      if (latestRolesShown.current !== rolesShown) return;
      await persistMatchRoles(resolveEffectiveMatchRoles(startingPlayers, matchRoles, picks));
    } catch (error) {
      console.error("Failed to read the automatic set-piece takers:", error);
    }
  }

  const [rolesOpen, setRolesOpen] = useState(true);
  const [blueprintOpen, setBlueprintOpen] = useState(true);

  return (
    <div className="flex flex-col gap-4">
      {/* Roles section */}
      <div className="rounded-xl border border-gray-200 bg-white dark:border-navy-600 dark:bg-navy-800">
        <div className="border-b border-gray-100 px-3 py-2 dark:border-navy-700">
          <button
            type="button"
            onClick={() => {
              setRolesOpen((o) => !o);
            }}
            aria-expanded={rolesOpen}
            className="flex items-center gap-1.5 text-[11px] font-heading font-bold uppercase tracking-[0.22em] text-gray-500 hover:text-gray-700 dark:text-gray-400 dark:hover:text-gray-300"
          >
            <ChevronDown
              className={`h-3 w-3 transition-transform duration-150 ${rolesOpen ? "" : "-rotate-90"}`}
            />
            {t("tactics.teamRoles")}
          </button>
        </div>
        {rolesOpen && (
          <div className="p-3">
            {startingPlayers.length === 0 ? (
              <p className="py-4 text-center text-xs text-gray-500 dark:text-gray-400">
                {t("tactics.noStartersForRoles")}
              </p>
            ) : (
              <div className="space-y-1">
                <button
                  type="button"
                  onClick={() => {
                    void handleAutoSelectAssignments();
                  }}
                  className="mb-2 w-full rounded-lg border border-primary-200 py-1.5 text-[10px] font-heading font-bold uppercase tracking-[0.18em] text-primary-500 transition-colors hover:bg-primary-50 dark:border-primary-500/30 dark:text-primary-400 dark:hover:bg-primary-500/10"
                >
                  {t("tactics.autoSelectAssignments")}
                </button>
                <SetPieceSelector
                  label={t("preMatch.captain")}
                  icon={<Crown className="h-4 w-4" />}
                  assignment="captain"
                  currentId={effectiveRoles.captain}
                  players={selectorPlayers}
                  allSquad={allSquad}
                  onSelect={(id) => {
                    void handleRoleChange("captain", id);
                  }}
                />
                <SetPieceSelector
                  label={t("tactics.viceCaptain")}
                  icon={<Award className="h-4 w-4" />}
                  assignment="vicecaptain"
                  currentId={effectiveRoles.vice_captain}
                  players={selectorPlayers}
                  allSquad={allSquad}
                  onSelect={(id) => {
                    void handleRoleChange("vice_captain", id);
                  }}
                />
                <SetPieceSelector
                  label={t("preMatch.penaltyTaker")}
                  icon={<CircleDot className="h-4 w-4" />}
                  assignment="penalty"
                  currentId={effectiveRoles.penalty_taker}
                  players={selectorPlayers.filter((p) => p.position !== "Goalkeeper")}
                  allSquad={allSquad}
                  onSelect={(id) => {
                    void handleRoleChange("penalty_taker", id);
                  }}
                />
                <SetPieceSelector
                  label={t("preMatch.freeKickTaker")}
                  icon={<Footprints className="h-4 w-4" />}
                  assignment="freekick"
                  currentId={effectiveRoles.free_kick_taker}
                  players={selectorPlayers.filter((p) => p.position !== "Goalkeeper")}
                  allSquad={allSquad}
                  onSelect={(id) => {
                    void handleRoleChange("free_kick_taker", id);
                  }}
                />
                <SetPieceSelector
                  label={t("preMatch.cornerTaker")}
                  icon={<CornerDownRight className="h-4 w-4" />}
                  assignment="corner"
                  currentId={effectiveRoles.corner_taker}
                  players={selectorPlayers.filter((p) => p.position !== "Goalkeeper")}
                  allSquad={allSquad}
                  onSelect={(id) => {
                    void handleRoleChange("corner_taker", id);
                  }}
                />
              </div>
            )}
          </div>
        )}
      </div>

      {/* Phase Blueprint section */}
      <div className="rounded-xl border border-gray-200 bg-white dark:border-navy-600 dark:bg-navy-800">
        <div className="border-b border-gray-100 px-3 py-2 dark:border-navy-700">
          <button
            type="button"
            onClick={() => {
              setBlueprintOpen((o) => !o);
            }}
            aria-expanded={blueprintOpen}
            className="flex items-center gap-1.5 text-[11px] font-heading font-bold uppercase tracking-[0.22em] text-gray-500 hover:text-gray-700 dark:text-gray-400 dark:hover:text-gray-300"
          >
            <ChevronDown
              className={`h-3 w-3 transition-transform duration-150 ${blueprintOpen ? "" : "-rotate-90"}`}
            />
            {t("tactics.phaseBlueprint")}
          </button>
        </div>
        {blueprintOpen && (
          <PhaseBlueprintPanel
            tacticsPhase={tacticsPhase}
            onTacticsPhaseChange={onTacticsPhaseChange}
          />
        )}
      </div>
    </div>
  );
}
