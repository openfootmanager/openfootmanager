import type {
  GameStateData,
  LoanOfferData,
  GamePlayerData,
  PlayerSelectionOptions,
} from "../../store/gameStore";
export interface TransfersTabProps {
  gameState: GameStateData;
  onSelectPlayer: (id: string, options?: PlayerSelectionOptions) => void;
  onSelectTeam: (id: string) => void;
  onGameUpdate?: (game: GameStateData) => void;
}
export type CounterTarget = {
  player: GamePlayerData;
  offerId: string;
  fromTeamId: string;
  fee: number;
};
export type LoanCounterTarget = {
  player: GamePlayerData;
  offer: LoanOfferData;
};
