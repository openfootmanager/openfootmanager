import { GameStateData, PlayerSelectionOptions } from "../../store/gameStore";
import SquadRosterView, {
  type SquadRosterSortState,
} from "./SquadRosterView";

interface SquadTabProps {
  gameState: GameStateData;
  managerId: string;
  onSelectPlayer: (id: string, options?: PlayerSelectionOptions) => void;
  onGameUpdate?: (g: GameStateData) => void;
  sortState?: SquadRosterSortState;
  onSortStateChange?: (sortState: SquadRosterSortState) => void;
}

export default function SquadTab({
  gameState,
  managerId,
  onSelectPlayer,
  onGameUpdate,
  sortState,
  onSortStateChange,
}: SquadTabProps) {
  return (
    <SquadRosterView
      gameState={gameState}
      managerId={managerId}
      onSelectPlayer={onSelectPlayer}
      onGameUpdate={onGameUpdate}
      sortState={sortState}
      onSortStateChange={onSortStateChange}
    />
  );
}
