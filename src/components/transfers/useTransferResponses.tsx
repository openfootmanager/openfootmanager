import { useState } from "react";
import type { TFunction } from "i18next";
import type { GameStateData } from "../../store/gameStore";
import {
  exerciseLoanBuyOption,
  respondToOffer,
  respondToLoanOffer,
} from "../../services/transfersService";
import { getErrorMessage, resolveTranslatedErrorMessage } from "../../utils/errorMessage";
interface Input {
  onGameUpdate: ((game: GameStateData) => void) | undefined;
  t: TFunction;
}
export function useTransferResponses({ onGameUpdate, t }: Input) {
  const [responseError, setResponseError] = useState<string | null>(null);
  const respond = async (
    failureLog: string,
    action: () => Promise<GameStateData>,
  ): Promise<void> => {
    setResponseError(null);
    try {
      const game = await action();
      if (onGameUpdate) onGameUpdate(game);
    } catch (err) {
      console.error(failureLog, err);
      setResponseError(resolveTranslatedErrorMessage(getErrorMessage(err), t));
    }
  };
  const handleRespondOffer = (playerId: string, offerId: string, accept: boolean) =>
    respond("Failed to respond to offer:", () => respondToOffer(playerId, offerId, accept));
  const handleRespondLoanOffer = (playerId: string, offerId: string, accept: boolean) =>
    respond("Failed to respond to loan offer:", () =>
      respondToLoanOffer(playerId, offerId, accept),
    );
  const handleExerciseLoanBuyOption = (playerId: string) =>
    respond("Failed to exercise loan buy option:", () => exerciseLoanBuyOption(playerId));
  return {
    responseError,
    handleRespondOffer,
    handleRespondLoanOffer,
    handleExerciseLoanBuyOption,
  };
}
