import type { TransfersTabProps } from "./TransfersTab.types";
import { useTransferBidFlow } from "./useTransferBidFlow";
import { useTransferWindow } from "./useTransferWindow";
import { useTransferMarket } from "./useTransferMarket";
import { useLoanOfferFlow } from "./useLoanOfferFlow";
import { useLoanCounterFlow } from "./useLoanCounterFlow";
import { useTransferCounterFlow } from "./useTransferCounterFlow";
import { useTransferResponses } from "./useTransferResponses";
import { useTransferActions } from "./useTransferActions";
import { useDealWorkspaceFlow } from "./useDealWorkspaceFlow";
export function useTransfersTabController({
  gameState,
  onSelectPlayer,
  onSelectTeam,
  onGameUpdate,
}: TransfersTabProps) {
  const transferWindow = useTransferWindow({ gameState });
  const { t, userTeamId, loanRegistrationDate, transferWindowBlocksRegistration } = transferWindow;
  const transferBidFlow = useTransferBidFlow({ gameState, onGameUpdate });
  const { myTeam, closeBidNegotiation, openBidNegotiation, bidLoading } = transferBidFlow;
  const transferMarket = useTransferMarket({ gameState, userTeamId, t, myTeam });
  const loanOfferFlow = useLoanOfferFlow({
    loanRegistrationDate,
    onGameUpdate,
    t,
    transferWindowBlocksRegistration,
  });
  const { loanLoading, openLoanOffer, closeLoanOffer } = loanOfferFlow;
  const loanCounterFlow = useLoanCounterFlow({
    loanRegistrationDate,
    onGameUpdate,
    t,
    transferWindowBlocksRegistration,
  });
  const transferCounterFlow = useTransferCounterFlow({ onGameUpdate, t });
  const transferResponses = useTransferResponses({ onGameUpdate, t });
  const transferActions = useTransferActions({ gameState, userTeamId, onGameUpdate, t });
  const dealWorkspaceFlow = useDealWorkspaceFlow({
    gameState,
    onGameUpdate,
    transferWindowBlocksRegistration,
    closeBidNegotiation,
    closeLoanOffer,
    openLoanOffer,
    openBidNegotiation,
    bidLoading,
    loanLoading,
    t,
  });
  return {
    gameState,
    onSelectPlayer,
    onSelectTeam,
    ...transferWindow,
    ...transferBidFlow,
    ...transferMarket,
    ...loanOfferFlow,
    ...loanCounterFlow,
    ...transferCounterFlow,
    ...transferResponses,
    ...transferActions,
    ...dealWorkspaceFlow,
  };
}
export type TransfersTabController = ReturnType<typeof useTransfersTabController>;
