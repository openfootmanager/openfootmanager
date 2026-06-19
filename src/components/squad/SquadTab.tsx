import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import {
  type GameStateData,
  type PlayerData,
  useGameStore,
} from "../../store/gameStore";
import type { PlayerSelectionOptions } from "../../store/gameStore";
import { getSquad } from "../../services/squadService";
import SquadRosterView from "./SquadRosterView";

interface SquadTabProps {
  gameState: GameStateData | null;
  managerId: string;
  onSelectPlayer: (id: string, options?: PlayerSelectionOptions) => void;
  onGameUpdate?: (g: GameStateData) => void;
}

export default function SquadTab({
  gameState,
  managerId,
  onSelectPlayer,
  onGameUpdate,
}: SquadTabProps) {
  const { t } = useTranslation();
  const { sessionState } = useGameStore();
  const [fetchedSquad, setFetchedSquad] = useState<PlayerData[] | null>(null);

  const teamId =
    sessionState?.manager?.team_id ?? gameState?.manager?.team_id ?? null;

  useEffect(() => {
    if (!teamId) return;
    void getSquad(teamId)
      .then(setFetchedSquad)
      .catch(() => {});
  }, [teamId]);

  const team =
    sessionState?.team ??
    gameState?.teams.find((t) => t.manager_id === managerId) ??
    null;
  const clockDate =
    sessionState?.clock.current_date ?? gameState?.clock.current_date ?? "";
  const players =
    fetchedSquad ??
    gameState?.players.filter((p) => p.team_id === teamId) ??
    [];

  const handleMutationComplete = (game: GameStateData) => {
    onGameUpdate?.(game);
    if (teamId) {
      setFetchedSquad(game.players.filter((p) => p.team_id === teamId));
    }
  };

  if (!team) {
    return (
      <p className="text-gray-500 dark:text-gray-400">
        {t("common.unemployed")}
      </p>
    );
  }

  return (
    <SquadRosterView
      players={players}
      team={team}
      clockDate={clockDate}
      onSelectPlayer={onSelectPlayer}
      onMutationComplete={handleMutationComplete}
    />
  );
}
