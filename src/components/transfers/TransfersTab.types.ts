import type {
  GameStateData,
  LoanOfferData,
  PlayerData,
  PlayerSelectionOptions,
} from "../../store/gameStore";
export interface TransfersTabProps {
  gameState: GameStateData;
  onSelectPlayer: (id: string, options?: PlayerSelectionOptions) => void;
  onSelectTeam: (id: string) => void;
  onGameUpdate?: (game: GameStateData) => void;
}
export type CounterTarget = {
  player: PlayerData;
  offerId: string;
  fromTeamId: string;
  fee: number;
};
export type LoanCounterTarget = {
  player: PlayerData;
  offer: LoanOfferData;
};
