export const ENVELOPE_WAGE_STEP_WEEKLY = 1_000;
export const ENVELOPE_TRANSFER_STEP = 52_000;

/** Whether raising the weekly wage cap by one step would still cover reserved transfer fees. */
export function canRaiseWageBudget(transferBudget: number, committedTransferFees: number): boolean {
  const nextTransfer = transferBudget - ENVELOPE_TRANSFER_STEP;
  return nextTransfer >= 0 && nextTransfer >= committedTransferFees;
}

/** Whether returning one wage step to transfers would leave a non-negative wage cap. */
export function canRaiseTransferBudget(wageBudget: number): boolean {
  return wageBudget - ENVELOPE_WAGE_STEP_WEEKLY >= 0;
}
