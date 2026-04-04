import { useMemo } from "react";
import {
  Award,
  CircleDot,
  CornerDownRight,
  Crown,
  Footprints,
  Sparkles,
} from "lucide-react";
import { useTranslation } from "react-i18next";

import type {
  GameStateData,
  PlayerData,
  TeamMatchRolesData,
} from "../../store/types";
import SetPieceSelector from "../match/SetPieceSelector";
import { Card, CardBody, CardHeader } from "../ui";
import {
  applyTacticsRoleSelection,
  autoSelectTacticsAssignments,
} from "./TacticsRolesPanel.controller";
import {
  getEffectiveMatchRoles,
  getSelectorRolePlayers,
} from "./TacticsRolesPanel.helpers";

interface TacticsRolesPanelProps {
  allSquad: PlayerData[];
  matchRoles?: TeamMatchRolesData;
  onGameUpdate: (gameState: GameStateData) => void;
  startingPlayers: PlayerData[];
}

export default function TacticsRolesPanel({
  allSquad,
  matchRoles,
  onGameUpdate,
  startingPlayers,
}: TacticsRolesPanelProps) {
  const { t } = useTranslation();

  const selectorPlayers = useMemo(
    () => getSelectorRolePlayers(startingPlayers),
    [startingPlayers],
  );

  const effectiveRoles = useMemo(() => {
    return getEffectiveMatchRoles(startingPlayers, matchRoles);
  }, [matchRoles, startingPlayers]);

  async function handleRoleChange(
    role: keyof TeamMatchRolesData,
    playerId: string,
  ): Promise<void> {
    await applyTacticsRoleSelection(
      startingPlayers,
      effectiveRoles,
      role,
      playerId,
      onGameUpdate,
    );
  }

  async function handleAutoSelectAssignments(): Promise<void> {
    await autoSelectTacticsAssignments(effectiveRoles, onGameUpdate);
  }

  if (startingPlayers.length === 0) {
    return (
      <Card>
        <CardBody>
          <div className="py-10 text-center text-sm text-gray-500 dark:text-gray-400">
            {t(
              "tactics.noStartersForRoles",
              "Set your starting XI to assign team roles and set pieces.",
            )}
          </div>
        </CardBody>
      </Card>
    );
  }

  return (
    <div className="grid grid-cols-1 gap-4 xl:grid-cols-[minmax(0,1fr)_minmax(0,1fr)]">
      <Card>
        <CardHeader>{t("tactics.teamRoles", "Team Roles")}</CardHeader>
        <CardBody>
          <div className="mb-4 flex items-center justify-between gap-3 rounded-xl border border-gray-200 bg-gray-50 px-4 py-3 dark:border-navy-600 dark:bg-navy-800/70">
            <p className="text-sm text-gray-600 dark:text-gray-300">
              {t(
                "tactics.rolesHint",
                "Choose your leadership hierarchy and dead-ball specialists from the current Starting XI.",
              )}
            </p>
            <button
              type="button"
              onClick={() => {
                void handleAutoSelectAssignments();
              }}
              className="shrink-0 rounded-lg bg-primary-500 px-3 py-2 text-xs font-heading font-bold uppercase tracking-wider text-white transition-colors hover:bg-primary-400"
            >
              <span className="flex items-center gap-2">
                <Sparkles className="h-3.5 w-3.5" />
                {t("tactics.autoSelectAssignments", "Auto-select defaults")}
              </span>
            </button>
          </div>
          <SetPieceSelector
            label={t("match.captain")}
            icon={<Crown className="w-4 h-4 text-accent-400" />}
            role="captain"
            currentId={effectiveRoles.captain}
            players={selectorPlayers}
            allSquad={allSquad}
            onSelect={(id) => {
              void handleRoleChange("captain", id);
            }}
          />
          <SetPieceSelector
            label={t("tactics.viceCaptain", "Vice-captain")}
            icon={<Award className="w-4 h-4 text-accent-400" />}
            role="vicecaptain"
            currentId={effectiveRoles.vice_captain}
            players={selectorPlayers}
            allSquad={allSquad}
            onSelect={(id) => {
              void handleRoleChange("vice_captain", id);
            }}
          />
        </CardBody>
      </Card>
      <Card>
        <CardHeader>{t("tactics.setPiecesSection", "Set Pieces")}</CardHeader>
        <CardBody>
          <SetPieceSelector
            label={t("match.penaltyTaker")}
            icon={<CircleDot className="w-4 h-4 text-accent-400" />}
            role="penalty"
            currentId={effectiveRoles.penalty_taker}
            players={selectorPlayers}
            allSquad={allSquad}
            onSelect={(id) => {
              void handleRoleChange("penalty_taker", id);
            }}
          />
          <SetPieceSelector
            label={t("match.freeKickTaker")}
            icon={<Footprints className="w-4 h-4 text-accent-400" />}
            role="freekick"
            currentId={effectiveRoles.free_kick_taker}
            players={selectorPlayers}
            allSquad={allSquad}
            onSelect={(id) => {
              void handleRoleChange("free_kick_taker", id);
            }}
          />
          <SetPieceSelector
            label={t("match.cornerTaker")}
            icon={<CornerDownRight className="w-4 h-4 text-accent-400" />}
            role="corner"
            currentId={effectiveRoles.corner_taker}
            players={selectorPlayers}
            allSquad={allSquad}
            onSelect={(id) => {
              void handleRoleChange("corner_taker", id);
            }}
          />
        </CardBody>
      </Card>
    </div>
  );
}
