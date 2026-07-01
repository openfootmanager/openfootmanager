import { useTranslation } from "react-i18next";
import {
  type GameStateData,
  useGameStore,
} from "../../store/gameStore";
import type { PlayerSelectionOptions } from "../../store/gameStore";
import { useFetchedSquad } from "../../hooks/useFetchedSquad";
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

  const teamId =
    sessionState?.manager?.team_id ?? gameState?.manager?.team_id ?? null;
  const clockDate =
    sessionState?.clock.current_date ?? gameState?.clock.current_date ?? "";
  const [fetchedSquad, setFetchedSquad] = useFetchedSquad(teamId, clockDate);

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
