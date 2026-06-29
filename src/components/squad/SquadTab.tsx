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
import type { SquadListSortState } from "./SquadRosterView.state";

interface SquadTabProps {
  gameState: GameStateData | null;
  managerId: string;
  onSelectPlayer: (id: string, options?: PlayerSelectionOptions) => void;
  onGameUpdate?: (g: GameStateData) => void;
  sortState?: SquadListSortState;
  onSortStateChange?: (sortState: SquadListSortState) => void;
}

export default function SquadTab({
  gameState,
  managerId,
  onSelectPlayer,
  onGameUpdate,
  sortState,
  onSortStateChange,
}: SquadTabProps) {
  const { t } = useTranslation();
  const { sessionState } = useGameStore();
  const [fetchedSquad, setFetchedSquad] = useState<PlayerData[] | null>(null);

  const teamId =
    sessionState?.manager?.team_id ?? gameState?.manager?.team_id ?? null;
  const clockDate =
    sessionState?.clock.current_date ?? gameState?.clock.current_date ?? "";

  // Refetch when the team OR the game clock changes so condition/fitness and
  // other per-day fields refresh after advancing a day (not only on tab switch).
  useEffect(() => {
    if (!teamId) return;
    let cancelled = false;
    void getSquad(teamId)
      .then((squad) => {
        if (!cancelled) setFetchedSquad(squad);
      })
      .catch(() => {});
    return () => {
      cancelled = true;
    };
  }, [teamId, clockDate]);

  const team =
    sessionState?.team ??
    gameState?.teams.find((t) => t.manager_id === managerId) ??
    null;
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
      sortState={sortState}
      onSortStateChange={onSortStateChange}
    />
  );
}
