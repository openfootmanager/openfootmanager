import type { JSX } from "react";
import { useMemo, useRef } from "react";
import { Award, CircleDot, CornerDownRight, Crown, Footprints } from "lucide-react";
import { useTranslation } from "react-i18next";
import type { GameStateData, PlayerData, TeamMatchRolesData } from "../../store/gameStore";
import SetPieceSelector from "../match/SetPieceSelector";
import { Button } from "../ui";
import {
  buildUpdatedMatchRolesForAssignment,
  resolveEffectiveMatchRoles,
} from "./TacticsRoles.helpers";
import {
  type AutoSetPieces,
  autoSelectSetPieces,
  setTeamMatchRoles,
} from "../../services/squadService";

interface TacticsSetPiecesProps {
  allSquad: PlayerData[];
  /** The backend's takers for this eleven, `null` until it has answered. */
  autoSetPieces: AutoSetPieces | null;
  matchRoles?: TeamMatchRolesData;
  onGameUpdate: (g: GameStateData) => void;
  /** A write the backend refused: the screen tells the manager it did not save. */
  onWriteError: (error: unknown) => void;
  startingPlayers: PlayerData[];
}

/** The captain, the vice-captain and the set-piece takers, chosen from the eleven. */
export default function TacticsSetPieces({
  allSquad,
  autoSetPieces,
  matchRoles,
  onGameUpdate,
  onWriteError,
  startingPlayers,
}: TacticsSetPiecesProps): JSX.Element {
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
  const outfielders = selectorPlayers.filter((p) => p.position !== "Goalkeeper");

  const effectiveRoles = useMemo(
    () => resolveEffectiveMatchRoles(startingPlayers, matchRoles, autoSetPieces),
    [matchRoles, startingPlayers, autoSetPieces],
  );

  async function persistMatchRoles(nextRoles: TeamMatchRolesData): Promise<void> {
    try {
      const updated = await setTeamMatchRoles(nextRoles);
      onGameUpdate(updated);
    } catch (error) {
      console.error("Failed to set team match roles:", error);
      onWriteError(error);
    }
  }

  async function handleRoleChange(role: keyof TeamMatchRolesData, playerId: string): Promise<void> {
    await persistMatchRoles(
      buildUpdatedMatchRolesForAssignment(startingPlayers, matchRoles, role, playerId),
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
      onWriteError(error);
    }
  }

  if (startingPlayers.length === 0) {
    return (
      <p className="py-4 text-center text-body text-fg-muted">{t("tactics.noStartersForRoles")}</p>
    );
  }

  const takers = [
    ["captain", "preMatch.captain", <Crown className="h-4 w-4" />, "captain", selectorPlayers],
    [
      "vice_captain",
      "tactics.viceCaptain",
      <Award className="h-4 w-4" />,
      "vicecaptain",
      selectorPlayers,
    ],
    [
      "penalty_taker",
      "preMatch.penaltyTaker",
      <CircleDot className="h-4 w-4" />,
      "penalty",
      outfielders,
    ],
    [
      "free_kick_taker",
      "preMatch.freeKickTaker",
      <Footprints className="h-4 w-4" />,
      "freekick",
      outfielders,
    ],
    [
      "corner_taker",
      "preMatch.cornerTaker",
      <CornerDownRight className="h-4 w-4" />,
      "corner",
      outfielders,
    ],
  ] as const;

  return (
    <div className="flex flex-col gap-1">
      <Button
        variant="outline"
        size="sm"
        className="mb-2 w-full"
        onClick={() => {
          void handleAutoSelectAssignments();
        }}
      >
        {t("tactics.autoSelectAssignments")}
      </Button>
      {takers.map(([role, labelKey, icon, assignment, players]) => (
        <SetPieceSelector
          key={role}
          label={t(labelKey)}
          icon={icon}
          assignment={assignment}
          currentId={effectiveRoles[role]}
          players={players}
          allSquad={allSquad}
          onSelect={(id) => {
            void handleRoleChange(role, id);
          }}
        />
      ))}
    </div>
  );
}
