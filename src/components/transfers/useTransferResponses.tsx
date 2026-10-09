import type { GameStateData } from "../../store/gameStore";
import {
  exerciseLoanBuyOption,
  respondToOffer,
  respondToLoanOffer,
} from "../../services/transfersService";
interface Input {
  onGameUpdate: ((game: GameStateData) => void) | undefined;
}
export function useTransferResponses({ onGameUpdate }: Input) {
  const handleRespondOffer = async (playerId: string, offerId: string, accept: boolean) => {
    try {
      const game = await respondToOffer(playerId, offerId, accept);
      if (onGameUpdate) onGameUpdate(game);
    } catch (err) {
      console.error("Failed to respond to offer:", err);
    }
  };
  const handleRespondLoanOffer = async (playerId: string, offerId: string, accept: boolean) => {
    try {
      const game = await respondToLoanOffer(playerId, offerId, accept);
      if (onGameUpdate) onGameUpdate(game);
    } catch (err) {
      console.error("Failed to respond to loan offer:", err);
    }
  };
  const handleExerciseLoanBuyOption = async (playerId: string) => {
    try {
      const game = await exerciseLoanBuyOption(playerId);
      if (onGameUpdate) onGameUpdate(game);
    } catch (err) {
      console.error("Failed to exercise loan buy option:", err);
    }
  };
  return { handleRespondOffer, handleRespondLoanOffer, handleExerciseLoanBuyOption };
}
