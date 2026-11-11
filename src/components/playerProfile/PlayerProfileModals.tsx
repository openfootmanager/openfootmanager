import { useTranslation } from "react-i18next";

import type { GameStateData, PlayerData } from "../../store/gameStore";
import FreeAgentContractModal from "../transfers/FreeAgentContractModal";
import TransferBidModal from "../transfers/TransferBidModal";
import type { PlayerProfileActions } from "./usePlayerProfileActions";
import PlayerProfileRenewalModal from "./PlayerProfileRenewalModal";
import PlayerProfileTerminationModal from "./PlayerProfileTerminationModal";

interface PlayerProfileModalsProps {
  player: PlayerData;
  gameState: GameStateData;
  actions: PlayerProfileActions;
  hasAssistantManager: boolean;
}

/** The profile's dialogs: bidding, signing a free agent, renewing a contract and terminating it. */
export default function PlayerProfileModals({
  player,
  gameState,
  actions,
  hasAssistantManager,
}: PlayerProfileModalsProps) {
  const { t } = useTranslation();
  const { bid, freeAgent, renewal, contract } = actions;
  return (
    <>
      {bid.bidTarget && (
        <TransferBidModal
          bidTarget={bid.bidTarget}
          teams={gameState.teams}
          bidAmount={bid.bidAmount}
          onBidAmountChange={bid.setBidAmount}
          myTeam={bid.myTeam}
          bidFee={bid.bidFee}
          bidProjection={bid.bidProjection}
          bidFeedback={bid.bidFeedback}
          activeBidOffer={bid.activeBidOffer}
          hasExistingOffer={bid.hasExistingOffer}
          talksResumeOn={bid.talksResumeOn}
          bidResult={bid.bidResult}
          bidLoading={bid.bidLoading}
          bidSubmitDisabled={bid.bidSubmitDisabled}
          onSubmit={bid.handleMakeBid}
          onClose={bid.closeBidNegotiation}
        />
      )}
      {freeAgent.freeAgentTarget && (
        <FreeAgentContractModal
          player={freeAgent.freeAgentTarget}
          teams={gameState.teams}
          wage={freeAgent.contractWage}
          onWageChange={freeAgent.setContractWage}
          contractLength={freeAgent.contractLength}
          onContractLengthChange={freeAgent.setContractLength}
          projection={freeAgent.contractProjection}
          feedback={freeAgent.contractFeedback}
          statusMessage={freeAgent.contractStatusMessage(t)}
          statusClassName={freeAgent.contractStatusClassName}
          submitting={freeAgent.contractSubmitting}
          submitDisabled={freeAgent.contractSubmitDisabled}
          onSubmit={freeAgent.submitFreeAgentContract}
          onClose={freeAgent.closeFreeAgentContract}
        />
      )}

      <PlayerProfileRenewalModal
        show={renewal.showRenewalModal}
        playerName={player.full_name}
        t={t}
        renewalWage={renewal.renewalWage}
        renewalLength={renewal.renewalLength}
        renewalIsTerminal={renewal.renewalIsTerminal}
        isRenewalWageValid={renewal.isRenewalWageValid}
        renewalViolatesSoftCap={renewal.renewalViolatesSoftCap}
        renewalProjection={renewal.renewalProjection}
        renewalStatusMessage={renewal.renewalStatusMessage}
        renewalStatusClassName={renewal.renewalStatusClassName}
        renewalCooledOff={renewal.renewalCooledOff}
        renewalFeedback={renewal.renewalFeedback}
        renewalSubmitDisabled={renewal.renewalSubmitDisabled}
        delegateRenewalDisabled={!hasAssistantManager || renewal.renewalSubmitting}
        onWageChange={renewal.setRenewalWage}
        onLengthChange={renewal.setRenewalLength}
        onClose={renewal.closeRenewalModal}
        onDelegate={() => void renewal.handleDelegateRenewal()}
        onSubmit={() => void renewal.handleRenewalSubmit()}
      />

      <PlayerProfileTerminationModal
        show={contract.showTerminationModal}
        playerName={player.full_name}
        t={t}
        preview={contract.terminationPreview}
        errorMessage={contract.contractActionError}
        submitting={contract.contractActionSubmitting}
        onCancel={contract.closeTerminationModal}
        onConfirm={() => void contract.handleTerminateContract()}
      />
    </>
  );
}
