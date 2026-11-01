import { useTranslation } from "react-i18next";

import type { TeamData } from "../../store/gameStore";
import FreeAgentContractModal from "../transfers/FreeAgentContractModal";
import TransferBidModal from "../transfers/TransferBidModal";
import type { useFreeAgentContractFlow } from "../transfers/useFreeAgentContractFlow";
import type { useTransferBidFlow } from "../transfers/useTransferBidFlow";

interface PlayersDealModalsProps {
  teams: TeamData[];
  bidFlow: ReturnType<typeof useTransferBidFlow>;
  contractFlow: ReturnType<typeof useFreeAgentContractFlow>;
}

/** The transfer-bid and free-agent-contract dialogs a player's context menu can open. */
export default function PlayersDealModals({
  teams,
  bidFlow,
  contractFlow,
}: PlayersDealModalsProps) {
  const { t } = useTranslation();
  const {
    bidTarget,
    bidAmount,
    setBidAmount,
    bidResult,
    bidLoading,
    bidFeedback,
    bidProjection,
    bidFee,
    activeBidOffer,
    myTeam,
    hasExistingOffer,
    talksResumeOn,
    bidSubmitDisabled,
    closeBidNegotiation,
    handleMakeBid,
  } = bidFlow;
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
    closeFreeAgentContract,
    submitFreeAgentContract,
  } = contractFlow;
  return (
    <>
      {bidTarget && (
        <TransferBidModal
          bidTarget={bidTarget}
          teams={teams}
          bidAmount={bidAmount}
          onBidAmountChange={setBidAmount}
          myTeam={myTeam}
          bidFee={bidFee}
          bidProjection={bidProjection}
          bidFeedback={bidFeedback}
          activeBidOffer={activeBidOffer}
          hasExistingOffer={hasExistingOffer}
          talksResumeOn={talksResumeOn}
          bidResult={bidResult}
          bidLoading={bidLoading}
          bidSubmitDisabled={bidSubmitDisabled}
          onSubmit={handleMakeBid}
          onClose={closeBidNegotiation}
        />
      )}
      {freeAgentTarget && (
        <FreeAgentContractModal
          player={freeAgentTarget}
          teams={teams}
          wage={contractWage}
          onWageChange={setContractWage}
          contractLength={contractLength}
          onContractLengthChange={setContractLength}
          projection={contractProjection}
          feedback={contractFeedback}
          statusMessage={contractStatusMessage(t)}
          statusClassName={contractStatusClassName}
          submitting={contractSubmitting}
          submitDisabled={contractSubmitDisabled}
          onSubmit={submitFreeAgentContract}
          onClose={closeFreeAgentContract}
        />
      )}
    </>
  );
}
