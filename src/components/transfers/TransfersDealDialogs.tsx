import { formatDate } from "../../lib/dateFormatting";
import TransferBidModal, { TransferBidForm } from "./TransferBidModal";
import TransferCounterOfferModal from "./TransferCounterOfferModal";
import LoanOfferModal, { LoanOfferForm } from "./LoanOfferModal";
import PlayerDealWorkspace from "./PlayerDealWorkspace";
import FreeAgentContractModal, { FreeAgentContractForm } from "./FreeAgentContractModal";
import type { TransfersTabController } from "./useTransfersTabController";
type ActiveDealWorkspaceProps = {
  state: Pick<
    TransfersTabController,
    | "dealWorkspaceTarget"
    | "gameState"
    | "myTeam"
    | "weeklySuffix"
    | "transferWindowBlocksRegistration"
    | "transferWindowSummary"
    | "loanWindowNoticeDetail"
    | "dealWorkspaceKind"
    | "contractWage"
    | "selectDealWorkspaceKind"
    | "closeDealWorkspace"
    | "bidTarget"
    | "bidAmount"
    | "setBidAmount"
    | "bidFee"
    | "bidProjection"
    | "bidFeedback"
    | "activeBidOffer"
    | "hasExistingOffer"
    | "bidResult"
    | "bidError"
    | "bidLoading"
    | "bidSubmitDisabled"
    | "transferWindowBlockingTitle"
    | "transferWindowBlockingDetail"
    | "handleMakeBid"
    | "loanTarget"
    | "loanPeriodId"
    | "loanPeriodOptions"
    | "selectedLoanPeriodOption"
    | "setLoanPeriodId"
    | "loanWageContributionPct"
    | "setLoanWageContributionPct"
    | "loanBuyOptionEnabled"
    | "loanBuyOptionFee"
    | "setLoanBuyOptionEnabled"
    | "setLoanBuyOptionFee"
    | "loanResult"
    | "loanSuggestedTerms"
    | "loanError"
    | "loanLoading"
    | "loanSubmitDisabled"
    | "loanWindowNoticeTitle"
    | "isTransferWindowClosed"
    | "closedWindowRegistrationDate"
    | "t"
    | "i18n"
    | "handleMakeLoanOffer"
    | "freeAgentTarget"
    | "setContractWage"
    | "contractLength"
    | "setContractLength"
    | "contractProjection"
    | "contractFeedback"
    | "contractStatusMessage"
    | "contractStatusClassName"
    | "contractSubmitting"
    | "contractSubmitDisabled"
    | "submitFreeAgentContract"
  >;
};
type TransferBidDialogProps = {
  state: Pick<
    TransfersTabController,
    | "bidTarget"
    | "dealWorkspaceTarget"
    | "gameState"
    | "bidAmount"
    | "setBidAmount"
    | "myTeam"
    | "bidFee"
    | "bidProjection"
    | "bidFeedback"
    | "activeBidOffer"
    | "hasExistingOffer"
    | "bidResult"
    | "bidError"
    | "bidLoading"
    | "transferWindowBlocksRegistration"
    | "bidSubmitDisabled"
    | "transferWindowBlockingTitle"
    | "transferWindowBlockingDetail"
    | "handleMakeBid"
    | "closeBidNegotiation"
  >;
};
type TransferCounterDialogProps = {
  state: Pick<
    TransfersTabController,
    | "counterTarget"
    | "gameState"
    | "counterAmount"
    | "setCounterAmount"
    | "counterFeedback"
    | "activeCounterOffer"
    | "counterResult"
    | "counterError"
    | "counterLoading"
    | "transferWindowBlocksRegistration"
    | "transferWindowBlockingTitle"
    | "transferWindowBlockingDetail"
    | "handleCounterOffer"
    | "setCounterTarget"
    | "setCounterError"
    | "setCounterResult"
    | "setCounterFeedback"
  >;
};
type FreeAgentDealDialogProps = {
  state: Pick<
    TransfersTabController,
    | "freeAgentTarget"
    | "dealWorkspaceTarget"
    | "gameState"
    | "contractWage"
    | "setContractWage"
    | "contractLength"
    | "setContractLength"
    | "contractProjection"
    | "contractFeedback"
    | "contractStatusMessage"
    | "t"
    | "contractStatusClassName"
    | "contractSubmitting"
    | "contractSubmitDisabled"
    | "submitFreeAgentContract"
    | "closeFreeAgentContract"
  >;
};
type LoanOfferDialogProps = {
  state: Pick<
    TransfersTabController,
    | "loanTarget"
    | "dealWorkspaceTarget"
    | "gameState"
    | "loanPeriodId"
    | "loanPeriodOptions"
    | "selectedLoanPeriodOption"
    | "setLoanPeriodId"
    | "loanWageContributionPct"
    | "setLoanWageContributionPct"
    | "loanBuyOptionEnabled"
    | "loanBuyOptionFee"
    | "setLoanBuyOptionEnabled"
    | "setLoanBuyOptionFee"
    | "loanResult"
    | "loanSuggestedTerms"
    | "loanError"
    | "loanLoading"
    | "loanSubmitDisabled"
    | "loanWindowNoticeTitle"
    | "loanWindowNoticeDetail"
    | "isTransferWindowClosed"
    | "closedWindowRegistrationDate"
    | "t"
    | "i18n"
    | "handleMakeLoanOffer"
    | "closeLoanOffer"
  >;
};
type LoanCounterDialogProps = {
  state: Pick<
    TransfersTabController,
    | "loanCounterTarget"
    | "gameState"
    | "loanCounterPeriodId"
    | "loanCounterPeriodOptions"
    | "selectedLoanCounterPeriodOption"
    | "setLoanCounterPeriodId"
    | "loanCounterWageContributionPct"
    | "setLoanCounterWageContributionPct"
    | "loanCounterBuyOptionEnabled"
    | "loanCounterBuyOptionFee"
    | "setLoanCounterBuyOptionEnabled"
    | "setLoanCounterBuyOptionFee"
    | "loanCounterResult"
    | "loanCounterSuggestedTerms"
    | "loanCounterError"
    | "loanCounterLoading"
    | "loanCounterSubmitDisabled"
    | "loanWindowNoticeTitle"
    | "loanWindowNoticeDetail"
    | "isTransferWindowClosed"
    | "closedWindowRegistrationDate"
    | "t"
    | "i18n"
    | "handleCounterLoanOffer"
    | "closeLoanCounterOffer"
  >;
};

export function ActiveDealWorkspace({ state }: ActiveDealWorkspaceProps) {
  const {
    dealWorkspaceTarget,
    gameState,
    myTeam,
    weeklySuffix,
    transferWindowBlocksRegistration,
    transferWindowSummary,
    loanWindowNoticeDetail,
    dealWorkspaceKind,
    contractWage,
    selectDealWorkspaceKind,
    closeDealWorkspace,
    bidTarget,
    bidAmount,
    setBidAmount,
    bidFee,
    bidProjection,
    bidFeedback,
    activeBidOffer,
    hasExistingOffer,
    bidResult,
    bidError,
    bidLoading,
    bidSubmitDisabled,
    transferWindowBlockingTitle,
    transferWindowBlockingDetail,
    handleMakeBid,
    loanTarget,
    loanPeriodId,
    loanPeriodOptions,
    selectedLoanPeriodOption,
    setLoanPeriodId,
    loanWageContributionPct,
    setLoanWageContributionPct,
    loanBuyOptionEnabled,
    loanBuyOptionFee,
    setLoanBuyOptionEnabled,
    setLoanBuyOptionFee,
    loanResult,
    loanSuggestedTerms,
    loanError,
    loanLoading,
    loanSubmitDisabled,
    loanWindowNoticeTitle,
    isTransferWindowClosed,
    closedWindowRegistrationDate,
    t,
    i18n,
    handleMakeLoanOffer,
    freeAgentTarget,
    setContractWage,
    contractLength,
    setContractLength,
    contractProjection,
    contractFeedback,
    contractStatusMessage,
    contractStatusClassName,
    contractSubmitting,
    contractSubmitDisabled,
    submitFreeAgentContract,
  } = state;
  return (
    dealWorkspaceTarget && (
      <PlayerDealWorkspace
        player={dealWorkspaceTarget}
        teams={gameState.teams}
        myTeam={myTeam ?? null}
        weeklySuffix={weeklySuffix}
        transferWindowBlocksRegistration={transferWindowBlocksRegistration}
        transferWindowSummary={transferWindowSummary}
        loanNoticeDetail={loanWindowNoticeDetail}
        selectedKind={dealWorkspaceKind}
        offeredWage={dealWorkspaceKind === "contract" ? Number(contractWage) : null}
        onSelectKind={(kind) => selectDealWorkspaceKind(dealWorkspaceTarget, kind)}
        onClose={closeDealWorkspace}
        renderDealPanel={(kind) => {
          if (kind === "transfer" && bidTarget) {
            return (
              <TransferBidForm
                bidTarget={bidTarget}
                teams={gameState.teams}
                bidAmount={bidAmount}
                onBidAmountChange={setBidAmount}
                myTeam={myTeam ?? null}
                bidFee={bidFee}
                bidProjection={bidProjection}
                bidFeedback={bidFeedback}
                activeBidOffer={activeBidOffer}
                hasExistingOffer={hasExistingOffer}
                bidResult={bidResult}
                bidError={bidError}
                bidLoading={bidLoading}
                bidSubmitDisabled={transferWindowBlocksRegistration || bidSubmitDisabled}
                blockingTitle={transferWindowBlockingTitle}
                blockingDetail={transferWindowBlockingDetail}
                showPlayerSummary={false}
                onSubmit={handleMakeBid}
                onClose={closeDealWorkspace}
              />
            );
          }

          if (kind === "loan" && loanTarget) {
            return (
              <LoanOfferForm
                loanTarget={loanTarget}
                teams={gameState.teams}
                periodId={loanPeriodId}
                periodOptions={loanPeriodOptions}
                selectedEndDate={selectedLoanPeriodOption?.endDate ?? ""}
                onPeriodChange={setLoanPeriodId}
                wageContributionPct={loanWageContributionPct}
                onWageContributionChange={setLoanWageContributionPct}
                buyOptionEnabled={loanBuyOptionEnabled}
                buyOptionFee={loanBuyOptionFee}
                onBuyOptionEnabledChange={setLoanBuyOptionEnabled}
                onBuyOptionFeeChange={setLoanBuyOptionFee}
                result={loanResult}
                suggestedTerms={loanSuggestedTerms}
                error={loanError}
                loading={loanLoading}
                submitDisabled={loanSubmitDisabled}
                noticeTitle={loanWindowNoticeTitle}
                noticeDetail={loanWindowNoticeDetail}
                acceptedMessage={
                  isTransferWindowClosed && closedWindowRegistrationDate
                    ? t("transfers.loanOfferScheduled", {
                        date: formatDate(closedWindowRegistrationDate, i18n.language),
                      })
                    : null
                }
                showPlayerSummary={false}
                onSubmit={handleMakeLoanOffer}
                onClose={closeDealWorkspace}
              />
            );
          }

          if (kind === "contract" && freeAgentTarget) {
            return (
              <FreeAgentContractForm
                player={freeAgentTarget}
                teams={gameState.teams}
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
                showPlayerSummary={false}
                onSubmit={submitFreeAgentContract}
                onClose={closeDealWorkspace}
              />
            );
          }

          return (
            <div className="rounded-lg bg-gray-50 p-6 text-sm text-gray-600 dark:bg-navy-900/50 dark:text-gray-300">
              {t("transfers.dealChooserHint")}
            </div>
          );
        }}
      />
    )
  );
}

export function TransferBidDialog({ state }: TransferBidDialogProps) {
  const {
    bidTarget,
    dealWorkspaceTarget,
    gameState,
    bidAmount,
    setBidAmount,
    myTeam,
    bidFee,
    bidProjection,
    bidFeedback,
    activeBidOffer,
    hasExistingOffer,
    bidResult,
    bidError,
    bidLoading,
    transferWindowBlocksRegistration,
    bidSubmitDisabled,
    transferWindowBlockingTitle,
    transferWindowBlockingDetail,
    handleMakeBid,
    closeBidNegotiation,
  } = state;
  return (
    bidTarget &&
    !dealWorkspaceTarget && (
      <TransferBidModal
        bidTarget={bidTarget}
        teams={gameState.teams}
        bidAmount={bidAmount}
        onBidAmountChange={setBidAmount}
        myTeam={myTeam ?? null}
        bidFee={bidFee}
        bidProjection={bidProjection}
        bidFeedback={bidFeedback}
        activeBidOffer={activeBidOffer}
        hasExistingOffer={hasExistingOffer}
        bidResult={bidResult}
        bidError={bidError}
        bidLoading={bidLoading}
        bidSubmitDisabled={transferWindowBlocksRegistration || bidSubmitDisabled}
        blockingTitle={transferWindowBlockingTitle}
        blockingDetail={transferWindowBlockingDetail}
        onSubmit={handleMakeBid}
        onClose={closeBidNegotiation}
      />
    )
  );
}

export function TransferCounterDialog({ state }: TransferCounterDialogProps) {
  const {
    counterTarget,
    gameState,
    counterAmount,
    setCounterAmount,
    counterFeedback,
    activeCounterOffer,
    counterResult,
    counterError,
    counterLoading,
    transferWindowBlocksRegistration,
    transferWindowBlockingTitle,
    transferWindowBlockingDetail,
    handleCounterOffer,
    setCounterTarget,
    setCounterError,
    setCounterResult,
    setCounterFeedback,
  } = state;
  return (
    counterTarget && (
      <TransferCounterOfferModal
        counterTarget={counterTarget}
        teams={gameState.teams}
        counterAmount={counterAmount}
        onCounterAmountChange={setCounterAmount}
        counterFeedback={counterFeedback}
        activeCounterOffer={activeCounterOffer}
        counterResult={counterResult}
        counterError={counterError}
        counterLoading={counterLoading}
        submitDisabled={transferWindowBlocksRegistration}
        blockingTitle={transferWindowBlockingTitle}
        blockingDetail={transferWindowBlockingDetail}
        onSubmit={handleCounterOffer}
        onClose={() => {
          setCounterTarget(null);
          setCounterAmount("");
          setCounterError(null);
          setCounterResult(null);
          setCounterFeedback(null);
        }}
      />
    )
  );
}

export function FreeAgentDealDialog({ state }: FreeAgentDealDialogProps) {
  const {
    freeAgentTarget,
    dealWorkspaceTarget,
    gameState,
    contractWage,
    setContractWage,
    contractLength,
    setContractLength,
    contractProjection,
    contractFeedback,
    contractStatusMessage,
    t,
    contractStatusClassName,
    contractSubmitting,
    contractSubmitDisabled,
    submitFreeAgentContract,
    closeFreeAgentContract,
  } = state;
  return (
    freeAgentTarget &&
    !dealWorkspaceTarget && (
      <FreeAgentContractModal
        player={freeAgentTarget}
        teams={gameState.teams}
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
    )
  );
}

export function LoanOfferDialog({ state }: LoanOfferDialogProps) {
  const {
    loanTarget,
    dealWorkspaceTarget,
    gameState,
    loanPeriodId,
    loanPeriodOptions,
    selectedLoanPeriodOption,
    setLoanPeriodId,
    loanWageContributionPct,
    setLoanWageContributionPct,
    loanBuyOptionEnabled,
    loanBuyOptionFee,
    setLoanBuyOptionEnabled,
    setLoanBuyOptionFee,
    loanResult,
    loanSuggestedTerms,
    loanError,
    loanLoading,
    loanSubmitDisabled,
    loanWindowNoticeTitle,
    loanWindowNoticeDetail,
    isTransferWindowClosed,
    closedWindowRegistrationDate,
    t,
    i18n,
    handleMakeLoanOffer,
    closeLoanOffer,
  } = state;
  return (
    loanTarget &&
    !dealWorkspaceTarget && (
      <LoanOfferModal
        loanTarget={loanTarget}
        teams={gameState.teams}
        periodId={loanPeriodId}
        periodOptions={loanPeriodOptions}
        selectedEndDate={selectedLoanPeriodOption?.endDate ?? ""}
        onPeriodChange={setLoanPeriodId}
        wageContributionPct={loanWageContributionPct}
        onWageContributionChange={setLoanWageContributionPct}
        buyOptionEnabled={loanBuyOptionEnabled}
        buyOptionFee={loanBuyOptionFee}
        onBuyOptionEnabledChange={setLoanBuyOptionEnabled}
        onBuyOptionFeeChange={setLoanBuyOptionFee}
        result={loanResult}
        suggestedTerms={loanSuggestedTerms}
        error={loanError}
        loading={loanLoading}
        submitDisabled={loanSubmitDisabled}
        noticeTitle={loanWindowNoticeTitle}
        noticeDetail={loanWindowNoticeDetail}
        acceptedMessage={
          isTransferWindowClosed && closedWindowRegistrationDate
            ? t("transfers.loanOfferScheduled", {
                date: formatDate(closedWindowRegistrationDate, i18n.language),
              })
            : null
        }
        onSubmit={handleMakeLoanOffer}
        onClose={closeLoanOffer}
      />
    )
  );
}

export function LoanCounterDialog({ state }: LoanCounterDialogProps) {
  const {
    loanCounterTarget,
    gameState,
    loanCounterPeriodId,
    loanCounterPeriodOptions,
    selectedLoanCounterPeriodOption,
    setLoanCounterPeriodId,
    loanCounterWageContributionPct,
    setLoanCounterWageContributionPct,
    loanCounterBuyOptionEnabled,
    loanCounterBuyOptionFee,
    setLoanCounterBuyOptionEnabled,
    setLoanCounterBuyOptionFee,
    loanCounterResult,
    loanCounterSuggestedTerms,
    loanCounterError,
    loanCounterLoading,
    loanCounterSubmitDisabled,
    loanWindowNoticeTitle,
    loanWindowNoticeDetail,
    isTransferWindowClosed,
    closedWindowRegistrationDate,
    t,
    i18n,
    handleCounterLoanOffer,
    closeLoanCounterOffer,
  } = state;
  return (
    loanCounterTarget && (
      <LoanOfferModal
        loanTarget={loanCounterTarget.player}
        teams={gameState.teams}
        periodId={loanCounterPeriodId}
        periodOptions={loanCounterPeriodOptions}
        selectedEndDate={selectedLoanCounterPeriodOption?.endDate ?? ""}
        onPeriodChange={setLoanCounterPeriodId}
        wageContributionPct={loanCounterWageContributionPct}
        onWageContributionChange={setLoanCounterWageContributionPct}
        buyOptionEnabled={loanCounterBuyOptionEnabled}
        buyOptionFee={loanCounterBuyOptionFee}
        onBuyOptionEnabledChange={setLoanCounterBuyOptionEnabled}
        onBuyOptionFeeChange={setLoanCounterBuyOptionFee}
        result={loanCounterResult}
        titleKey="transfers.counterLoanOffer"
        submitLabelKey="transfers.submitLoanCounter"
        acceptedLabelKey="transfers.loanCounterAccepted"
        rejectedLabelKey="transfers.loanCounterRejected"
        counteredLabelKey="transfers.loanCounterCountered"
        suggestedTerms={loanCounterSuggestedTerms}
        error={loanCounterError}
        loading={loanCounterLoading}
        submitDisabled={loanCounterSubmitDisabled}
        noticeTitle={loanWindowNoticeTitle}
        noticeDetail={loanWindowNoticeDetail}
        acceptedMessage={
          isTransferWindowClosed && closedWindowRegistrationDate
            ? t("transfers.loanCounterScheduled", {
                date: formatDate(closedWindowRegistrationDate, i18n.language),
              })
            : null
        }
        onSubmit={handleCounterLoanOffer}
        onClose={closeLoanCounterOffer}
      />
    )
  );
}
