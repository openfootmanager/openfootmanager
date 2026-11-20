import { fireEvent, render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";

import { formatExactMoney, formatWeeklyWage } from "../../lib/helpers";
import { FinanceEnvelopeCard } from "./FinanceEnvelopeCard";
import { ENVELOPE_TRANSFER_STEP, ENVELOPE_WAGE_STEP_WEEKLY } from "./FinanceEnvelopeCard.helpers";
import type { FinanceViewState } from "./useFinanceView";

function renderCard(
  overrides: {
    wageBudget?: number;
    transferBudget?: number;
    committed?: number;
    onReallocate?: (wageDeltaWeekly: number) => Promise<void>;
    busy?: boolean;
    previewsCurrent?: boolean;
  } = {},
) {
  const handleReallocateBoardEnvelopes = overrides.onReallocate ?? vi.fn(async () => {});
  const t = ((key: string, params?: Record<string, string>) =>
    params
      ? `${key} ${params.wage ?? ""} ${params.transfer ?? ""}`.trim()
      : key) as FinanceViewState["t"];
  render(
    <FinanceEnvelopeCard
      state={{
        t,
        financeSnapshot: {
          committedTransferFees: overrides.committed ?? 0,
        } as never,
        myTeam: {
          wage_budget: overrides.wageBudget ?? 120_000,
          transfer_budget: overrides.transferBudget ?? 300_000,
        } as never,
        reallocationLoading: overrides.busy ?? false,
        previewsCurrent: overrides.previewsCurrent ?? true,
        handleReallocateBoardEnvelopes,
        reallocationFeedback: null,
      }}
    />,
  );
  return handleReallocateBoardEnvelopes;
}

describe("FinanceEnvelopeCard", () => {
  /** Given current wage and transfer limits, when the card renders, then both amounts are exact. */
  it("shows exact wage and transfer limits", () => {
    renderCard({ wageBudget: 120_000, transferBudget: 300_000 });
    expect(screen.getByText(formatWeeklyWage(120_000))).toBeInTheDocument();
    expect(screen.getByText(formatExactMoney(300_000))).toBeInTheDocument();
    expect(
      screen.getByText(
        `finances.reallocateStep ${formatWeeklyWage(ENVELOPE_WAGE_STEP_WEEKLY)} ${formatExactMoney(ENVELOPE_TRANSFER_STEP)}`,
      ),
    ).toBeInTheDocument();
  });

  /** Given a wage raise is allowed, when the raise-wage control is activated, then +1000/wk is requested. */
  it("requests a weekly wage raise of one step", () => {
    const onReallocate = vi.fn(async () => {});
    renderCard({ onReallocate });
    fireEvent.click(screen.getByRole("button", { name: "finances.raiseWageBudget" }));
    expect(onReallocate).toHaveBeenCalledWith(ENVELOPE_WAGE_STEP_WEEKLY);
  });

  /** Given a transfer raise is allowed, when that control is activated, then −1000/wk is requested. */
  it("requests a transfer raise by returning one wage step", () => {
    const onReallocate = vi.fn(async () => {});
    renderCard({ onReallocate });
    fireEvent.click(screen.getByRole("button", { name: "finances.raiseTransferBudget" }));
    expect(onReallocate).toHaveBeenCalledWith(-ENVELOPE_WAGE_STEP_WEEKLY);
  });

  /** Given reserved fees a wage raise would uncover, when the card renders, then raise-wage is disabled. */
  it("disables a wage raise that would breach reserved fees", () => {
    renderCard({ transferBudget: 100_000, committed: 80_000 });
    expect(screen.getByRole("button", { name: "finances.raiseWageBudget" })).toBeDisabled();
    expect(screen.getByRole("button", { name: "finances.raiseTransferBudget" })).toBeEnabled();
  });

  /** Given a reallocation is already in flight, when the card renders, then both controls wait. */
  it("disables both controls while a reallocation is loading", () => {
    renderCard({ busy: true });
    expect(screen.getByRole("button", { name: "finances.raiseWageBudget" })).toBeDisabled();
    expect(screen.getByRole("button", { name: "finances.raiseTransferBudget" })).toBeDisabled();
  });

  /**
   * Given the retained snapshot describes an older club state
   * When a wage raise would otherwise be allowed from live limits plus that stale reserved-fee figure
   * Then raise-wage waits; raise-transfer still uses the live wage cap.
   */
  it("disables a wage raise while the finance snapshot is stale", () => {
    renderCard({ previewsCurrent: false, transferBudget: 300_000, committed: 0 });
    expect(screen.getByRole("button", { name: "finances.raiseWageBudget" })).toBeDisabled();
    expect(screen.getByRole("button", { name: "finances.raiseTransferBudget" })).toBeEnabled();
  });
});
