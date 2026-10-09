import { getPlayerDealBlocker } from "./TransfersTab.model";
import { useState } from "react";
import type { GameStateData, PlayerData } from "../../store/gameStore";
import { Handshake, ArrowRightLeft, Gavel, UserPlus } from "lucide-react";
import type { DealKind } from "./PlayerDealWorkspace";
import { useFreeAgentContractFlow } from "./useFreeAgentContractFlow";
import type { TFunction } from "i18next";
interface Input {
  gameState: GameStateData;
  onGameUpdate: ((game: GameStateData) => void) | undefined;
  transferWindowBlocksRegistration: boolean;
  closeBidNegotiation: () => void;
  closeLoanOffer: () => void;
  openLoanOffer: (player: PlayerData) => void;
  openBidNegotiation: (player: PlayerData) => void;
  bidLoading: boolean;
  loanLoading: boolean;
  t: TFunction<"translation", undefined>;
}
export function useDealWorkspaceFlow({
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
}: Input) {
  const [dealWorkspaceTargetId, setDealWorkspaceTargetId] = useState<string | null>(null);
  const dealWorkspaceTarget =
    gameState.players.find((player) => player.id === dealWorkspaceTargetId) ?? null;
  const [dealWorkspaceKind, setDealWorkspaceKind] = useState<DealKind>("transfer");
  const {
    freeAgentTarget,
    contractWage,
    setContractWage,
    contractLength,
    setContractLength,
    contractFeedback,
    contractProjection,
    contractSubmitting,
    contractSubmitDisabled,
    contractStatusMessage,
    contractStatusClassName,
    openFreeAgentContract,
    closeFreeAgentContract,
    submitFreeAgentContract,
  } = useFreeAgentContractFlow({
    gameState,
    onGameUpdate,
  });
  const getDealKinds = (player: PlayerData): DealKind[] => {
    const kinds: DealKind[] = [];

    if (player.team_id !== null && player.transfer_listed) {
      kinds.push("transfer");
    }

    if (player.team_id !== null && player.loan_listed) {
      kinds.push("loan");
    }

    if (player.team_id === null) {
      kinds.push("contract");
    }

    return kinds;
  };
  const getStartableDealKinds = (player: PlayerData): DealKind[] =>
    getDealKinds(player).filter((kind) => isDealKindStartable(player, kind));
  const isDealKindStartable = (player: PlayerData, kind: DealKind): boolean => {
    if (getPlayerDealBlocker(player, gameState.manager.team_id)) return false;
    if (kind === "transfer") {
      return player.team_id !== null && player.transfer_listed && !transferWindowBlocksRegistration;
    }

    if (kind === "loan") {
      return player.team_id !== null && player.loan_listed && !transferWindowBlocksRegistration;
    }

    return player.team_id === null;
  };
  const selectDealWorkspaceKind = (player: PlayerData, kind: DealKind) => {
    setDealWorkspaceKind(kind);

    if (kind === "contract") {
      closeBidNegotiation();
      closeLoanOffer();
      if (isDealKindStartable(player, kind)) {
        openFreeAgentContract(player);
      } else {
        closeFreeAgentContract();
      }
      return;
    }

    if (kind === "loan") {
      closeBidNegotiation();
      closeFreeAgentContract();
      if (isDealKindStartable(player, kind)) {
        openLoanOffer(player);
      } else {
        closeLoanOffer();
      }
      return;
    }

    closeLoanOffer();
    closeFreeAgentContract();
    if (isDealKindStartable(player, kind)) {
      openBidNegotiation(player);
    } else {
      closeBidNegotiation();
    }
  };
  const openDealEntry = (player: PlayerData) => {
    const dealKinds = getDealKinds(player);
    const startableDealKinds = getStartableDealKinds(player);
    const initialKind = startableDealKinds[0] ?? dealKinds[0] ?? "transfer";

    setDealWorkspaceTargetId(player.id);
    selectDealWorkspaceKind(player, initialKind);
  };
  const openDealHistory = (player: PlayerData) => {
    setDealWorkspaceTargetId(player.id);
    setDealWorkspaceKind(getDealKinds(player)[0] ?? "transfer");
    closeBidNegotiation();
    closeLoanOffer();
    closeFreeAgentContract();
  };

  const closeDealWorkspace = () => {
    if (bidLoading || loanLoading || contractSubmitting) {
      return;
    }

    setDealWorkspaceTargetId(null);
    closeBidNegotiation();
    closeLoanOffer();
    closeFreeAgentContract();
  };
  const getDealEntryLabel = (player: PlayerData): string => {
    const dealKinds = getDealKinds(player);

    if (dealKinds.length > 1) {
      return t("transfers.makeOffer");
    }

    if (dealKinds[0] === "contract") {
      return t("transfers.offerContract");
    }

    if (dealKinds[0] === "loan") {
      return t("transfers.loanOffer");
    }

    return t("transfers.bid");
  };
  const getDealEntryIcon = (player: PlayerData, className: string) => {
    const dealKinds = getDealKinds(player);

    if (dealKinds.length > 1) {
      return <Handshake className={className} />;
    }

    if (dealKinds[0] === "contract") {
      return <UserPlus className={className} />;
    }

    if (dealKinds[0] === "loan") {
      return <ArrowRightLeft className={className} />;
    }

    return <Gavel className={className} />;
  };
  return {
    dealWorkspaceTarget,
    dealWorkspaceKind,
    freeAgentTarget,
    contractWage,
    setContractWage,
    contractLength,
    setContractLength,
    contractFeedback,
    contractProjection,
    contractSubmitting,
    contractSubmitDisabled,
    contractStatusMessage,
    contractStatusClassName,
    closeFreeAgentContract,
    submitFreeAgentContract,
    selectDealWorkspaceKind,
    openDealEntry,
    openDealHistory,
    closeDealWorkspace,
    getDealEntryLabel,
    getDealEntryIcon,
  };
}
