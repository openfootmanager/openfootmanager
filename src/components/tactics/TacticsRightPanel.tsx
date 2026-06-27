import type { JSX } from "react";
import { useMemo, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { Award, ChevronDown, CircleDot, CornerDownRight, Crown, Footprints } from "lucide-react";
import { useTranslation } from "react-i18next";
import type { GameStateData, PlayerData, TeamMatchRolesData } from "../../store/gameStore";
import type { TacticsPhaseSettings } from "../../store/types";
import { Select } from "../ui";
import SetPieceSelector from "../match/SetPieceSelector";
import TacticsPlayerFocusPanel from "./TacticsPlayerFocusPanel";
import {
  buildUpdatedMatchRolesForAssignment,
  resolveEffectiveMatchRoles,
} from "./TacticsRoles.helpers";

interface TacticsRightPanelProps {
  allSquad: PlayerData[];
  canConfirmSwap: boolean;
  comparePlayer: PlayerData | null;
  matchRoles?: TeamMatchRolesData;
  onConfirmSwap: () => void;
  onGameUpdate: (g: GameStateData) => void;
  onTacticsPhaseChange: (patch: Partial<TacticsPhaseSettings>) => void;
  selectedPlayer: PlayerData | null;
  startingPlayers: PlayerData[];
  tacticsPhase?: TacticsPhaseSettings;
}

const WITH_BALL_FIELDS = [
  ["build_up_style", "buildUpStyle", ["Short", "Mixed", "Long"]] as const,
  ["width", "width", ["Narrow", "Normal", "Wide"]] as const,
  ["tempo", "tempo", ["Patient", "Direct"]] as const,
];

const WITHOUT_BALL_FIELDS = [
  ["defensive_line", "defensiveLine", ["VeryLow", "Low", "Medium", "High"]] as const,
  ["pressing_intensity", "pressingIntensity", ["Passive", "Medium", "Aggressive"]] as const,
  ["defensive_shape", "defensiveShape", ["Stretched", "Normal", "Compact"]] as const,
  ["marking_style", "markingStyle", ["Zonal", "Mixed", "ManToMan"]] as const,
];

const TRANSITION_FIELDS = [
  ["counter_press_duration", "counterPressDuration", ["None", "Short", "Long"]] as const,
  ["break_speed", "breakSpeed", ["Slow", "Medium", "Fast"]] as const,
];

function PhaseButtonGroup({
  field,
  labelKey,
  onTacticsPhaseChange,
  options,
  tacticsPhase,
}: {
  field: keyof TacticsPhaseSettings;
  labelKey: string;
  onTacticsPhaseChange: (patch: Partial<TacticsPhaseSettings>) => void;
  options: readonly string[];
  tacticsPhase?: TacticsPhaseSettings;
}): JSX.Element {
  const { t } = useTranslation();
  const currentValue = (tacticsPhase?.[field] ?? options[0]) as string;
  return (
    <div className="flex items-center gap-2">
      <span className="w-20 shrink-0 text-[10px] text-gray-500 dark:text-gray-400">
        {t(`tactics.phaseSettings.${labelKey}`)}
      </span>
      <Select
        selectSize="xs"
        variant="subtle"
        fullWidth
        value={currentValue}
        onChange={(e) => { onTacticsPhaseChange({ [field]: e.target.value }); }}
      >
        {options.map((opt) => (
          <option key={opt} value={opt}>
            {t(`tactics.phaseSettings.${labelKey}_${opt}`, opt)}
          </option>
        ))}
      </Select>
    </div>
  );
}

export default function TacticsRightPanel({
  allSquad,
  canConfirmSwap,
  comparePlayer,
  matchRoles,
  onConfirmSwap,
  onGameUpdate,
  onTacticsPhaseChange,
  selectedPlayer,
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
    () => resolveEffectiveMatchRoles(startingPlayers, matchRoles),
    [matchRoles, startingPlayers],
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

  async function handleRoleChange(
    role: keyof TeamMatchRolesData,
    playerId: string,
  ): Promise<void> {
    await persistMatchRoles(
      buildUpdatedMatchRolesForAssignment(
        effectiveRoles,
        startingPlayers,
        role,
        playerId,
      ),
    );
  }

  async function handleAutoSelectAssignments(): Promise<void> {
    await persistMatchRoles(effectiveRoles);
  }

  const [rolesOpen, setRolesOpen] = useState(true);
  const [blueprintOpen, setBlueprintOpen] = useState(true);

  return (
    <div className="flex flex-col gap-4">
      {/* Roles section */}
      <div className="rounded-xl border border-gray-200 bg-white dark:border-navy-600 dark:bg-navy-800">
        <div className="flex items-center justify-between border-b border-gray-100 px-3 py-2 dark:border-navy-700">
          <button
            type="button"
            onClick={() => { setRolesOpen((o) => !o); }}
            aria-expanded={rolesOpen}
            className="flex items-center gap-1.5 text-[10px] font-heading font-bold uppercase tracking-[0.22em] text-gray-500 hover:text-gray-700 dark:text-gray-400 dark:hover:text-gray-300"
          >
            <ChevronDown
              className={`h-3 w-3 transition-transform duration-150 ${rolesOpen ? "" : "-rotate-90"}`}
            />
            {t("tactics.teamRoles")}
          </button>
          {startingPlayers.length > 0 && (
            <button
              type="button"
              onClick={() => { void handleAutoSelectAssignments(); }}
              className="text-[10px] font-heading font-bold uppercase tracking-[0.18em] text-primary-500 hover:text-primary-600 dark:text-primary-400 dark:hover:text-primary-300"
            >
              {t("tactics.autoSelectAssignments")}
            </button>
          )}
        </div>
        {rolesOpen && (
          <div className="p-3">
            {startingPlayers.length === 0 ? (
              <p className="py-4 text-center text-xs text-gray-500 dark:text-gray-400">
                {t("tactics.noStartersForRoles")}
              </p>
            ) : (
              <div className="space-y-1">
                <SetPieceSelector
                  label={t("preMatch.captain")}
                  icon={<Crown className="h-4 w-4" />}
                  role="captain"
                  currentId={effectiveRoles.captain}
                  players={selectorPlayers}
                  allSquad={allSquad}
                  onSelect={(id) => { void handleRoleChange("captain", id); }}
                />
                <SetPieceSelector
                  label={t("tactics.viceCaptain")}
                  icon={<Award className="h-4 w-4" />}
                  role="vicecaptain"
                  currentId={effectiveRoles.vice_captain}
                  players={selectorPlayers}
                  allSquad={allSquad}
                  onSelect={(id) => { void handleRoleChange("vice_captain", id); }}
                />
                <SetPieceSelector
                  label={t("preMatch.penaltyTaker")}
                  icon={<CircleDot className="h-4 w-4" />}
                  role="penalty"
                  currentId={effectiveRoles.penalty_taker}
                  players={selectorPlayers.filter((p) => p.position !== "Goalkeeper")}
                  allSquad={allSquad}
                  onSelect={(id) => { void handleRoleChange("penalty_taker", id); }}
                />
                <SetPieceSelector
                  label={t("preMatch.freeKickTaker")}
                  icon={<Footprints className="h-4 w-4" />}
                  role="freekick"
                  currentId={effectiveRoles.free_kick_taker}
                  players={selectorPlayers.filter((p) => p.position !== "Goalkeeper")}
                  allSquad={allSquad}
                  onSelect={(id) => { void handleRoleChange("free_kick_taker", id); }}
                />
                <SetPieceSelector
                  label={t("preMatch.cornerTaker")}
                  icon={<CornerDownRight className="h-4 w-4" />}
                  role="corner"
                  currentId={effectiveRoles.corner_taker}
                  players={selectorPlayers.filter((p) => p.position !== "Goalkeeper")}
                  allSquad={allSquad}
                  onSelect={(id) => { void handleRoleChange("corner_taker", id); }}
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
            onClick={() => { setBlueprintOpen((o) => !o); }}
            aria-expanded={blueprintOpen}
            className="flex items-center gap-1.5 text-[10px] font-heading font-bold uppercase tracking-[0.22em] text-gray-500 hover:text-gray-700 dark:text-gray-400 dark:hover:text-gray-300"
          >
            <ChevronDown
              className={`h-3 w-3 transition-transform duration-150 ${blueprintOpen ? "" : "-rotate-90"}`}
            />
            {t("tactics.phaseBlueprint")}
          </button>
        </div>
        {blueprintOpen && (
          <div className="divide-y divide-gray-100 dark:divide-navy-700">
            <div className="p-3 space-y-2">
              <div className="mb-1.5 text-[10px] font-heading font-bold uppercase tracking-[0.2em] text-primary-500 dark:text-primary-400">
                {t("tactics.phaseLabels.withBall")}
              </div>
              {WITH_BALL_FIELDS.map(([field, labelKey, options]) => (
                <PhaseButtonGroup
                  key={field}
                  field={field}
                  labelKey={labelKey}
                  onTacticsPhaseChange={onTacticsPhaseChange}
                  options={options}
                  tacticsPhase={tacticsPhase}
                />
              ))}
            </div>
            <div className="p-3 space-y-2">
              <div className="mb-1.5 text-[10px] font-heading font-bold uppercase tracking-[0.2em] text-primary-500 dark:text-primary-400">
                {t("tactics.phaseLabels.withoutBall")}
              </div>
              {WITHOUT_BALL_FIELDS.map(([field, labelKey, options]) => (
                <PhaseButtonGroup
                  key={field}
                  field={field}
                  labelKey={labelKey}
                  onTacticsPhaseChange={onTacticsPhaseChange}
                  options={options}
                  tacticsPhase={tacticsPhase}
                />
              ))}
            </div>
            <div className="p-3 space-y-2">
              <div className="mb-1.5 text-[10px] font-heading font-bold uppercase tracking-[0.2em] text-primary-500 dark:text-primary-400">
                {t("tactics.phaseLabels.transitions")}
              </div>
              {TRANSITION_FIELDS.map(([field, labelKey, options]) => (
                <PhaseButtonGroup
                  key={field}
                  field={field}
                  labelKey={labelKey}
                  onTacticsPhaseChange={onTacticsPhaseChange}
                  options={options}
                  tacticsPhase={tacticsPhase}
                />
              ))}
            </div>
          </div>
        )}
      </div>

      {/* Player Focus panel */}
      {selectedPlayer && (
        <TacticsPlayerFocusPanel
          canConfirmSwap={canConfirmSwap}
          comparePlayer={comparePlayer}
          onConfirmSwap={onConfirmSwap}
          selectedPlayer={selectedPlayer}
        />
      )}
    </div>
  );
}
