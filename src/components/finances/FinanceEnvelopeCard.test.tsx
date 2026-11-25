import { fireEvent, render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";

import { formatExactMoney, formatWeeklyWage } from "../../lib/helpers";
import type { EnvelopeReallocationPreviewData } from "../../services/financeService";
import { FinanceEnvelopeCard } from "./FinanceEnvelopeCard";
import type { FinanceViewState } from "./useFinanceView";

const WAGE_STEP = 1_000;
const TRANSFER_STEP = 52_000;

function preview(
  overrides: Partial<EnvelopeReallocationPreviewData> & { wageDeltaWeekly: number },
): EnvelopeReallocationPreviewData {
  const transferDelta = -overrides.wageDeltaWeekly * 52;
  return {
    transferDelta,
    nextWageBudget: 121_000,
    nextTransferBudget: 248_000,
    allowed: true,
    blockReason: null,
    ...overrides,
  };
}

function renderCard(
  overrides: {
    wageBudget?: number;
    transferBudget?: number;
    onReallocate?: (wageDeltaWeekly: number) => Promise<void>;
    busy?: boolean;
    raiseWage?: EnvelopeReallocationPreviewData | null;
    raiseTransfer?: EnvelopeReallocationPreviewData | null;
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
        myTeam: {
          wage_budget: overrides.wageBudget ?? 120_000,
          transfer_budget: overrides.transferBudget ?? 300_000,
        } as never,
        reallocationLoading: overrides.busy ?? false,
        handleReallocateBoardEnvelopes,
        reallocationFeedback: null,
        raiseWagePreview:
          overrides.raiseWage === undefined
            ? preview({ wageDeltaWeekly: WAGE_STEP })
            : overrides.raiseWage,
        raiseTransferPreview:
          overrides.raiseTransfer === undefined
            ? preview({ wageDeltaWeekly: -WAGE_STEP })
            : overrides.raiseTransfer,
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
        `finances.reallocateStep ${formatWeeklyWage(WAGE_STEP)} ${formatExactMoney(TRANSFER_STEP)}`,
      ),
    ).toBeInTheDocument();
  });

  /** Given a wage raise is allowed, when the raise-wage control is activated, then the backend's weekly delta is requested. */
  it("requests a weekly wage raise of one step", () => {
    const onReallocate = vi.fn(async () => {});
    renderCard({ onReallocate });
    fireEvent.click(screen.getByRole("button", { name: "finances.raiseWageBudget" }));
    expect(onReallocate).toHaveBeenCalledWith(WAGE_STEP);
  });

  /** Given a transfer raise is allowed, when that control is activated, then the backend's negative weekly delta is requested. */
  it("requests a transfer raise by returning one wage step", () => {
    const onReallocate = vi.fn(async () => {});
    renderCard({ onReallocate });
    fireEvent.click(screen.getByRole("button", { name: "finances.raiseTransferBudget" }));
    expect(onReallocate).toHaveBeenCalledWith(-WAGE_STEP);
  });

  /** Given the backend refuses a wage raise because reserved fees would be uncovered, when the card renders, then raise-wage is disabled. */
  it("disables a wage raise that would breach reserved fees", () => {
    renderCard({
      raiseWage: preview({
        wageDeltaWeekly: WAGE_STEP,
        allowed: false,
        blockReason: "be.error.finance.reallocationBreachesReservations",
      }),
    });
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
   * When the backend's envelope previews are therefore withheld
   * Then both controls wait rather than guessing from live limits.
   */
  it("disables both envelope controls while the finance snapshot is stale", () => {
    renderCard({ raiseWage: null, raiseTransfer: null, transferBudget: 300_000 });
    expect(screen.getByRole("button", { name: "finances.raiseWageBudget" })).toBeDisabled();
    expect(screen.getByRole("button", { name: "finances.raiseTransferBudget" })).toBeDisabled();
  });
});
