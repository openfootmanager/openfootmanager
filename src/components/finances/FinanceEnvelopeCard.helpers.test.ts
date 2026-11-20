import { describe, expect, it } from "vitest";

import {
  ENVELOPE_TRANSFER_STEP,
  ENVELOPE_WAGE_STEP_WEEKLY,
  canRaiseTransferBudget,
  canRaiseWageBudget,
} from "./FinanceEnvelopeCard.helpers";

describe("FinanceEnvelopeCard helpers", () => {
  /** Given enough uncommitted transfer permission, when a wage raise is checked, then it is allowed. */
  it("allows a wage raise that still covers reserved fees", () => {
    expect(canRaiseWageBudget(300_000, 80_000)).toBe(true);
    expect(ENVELOPE_TRANSFER_STEP).toBe(ENVELOPE_WAGE_STEP_WEEKLY * 52);
  });

  /** Given reserved fees that a wage raise would uncover, when it is checked, then it is refused. */
  it("refuses a wage raise that would leave less than reserved fees", () => {
    expect(canRaiseWageBudget(100_000, 80_000)).toBe(false);
  });

  /** Given a wage cap smaller than one step, when a transfer raise is checked, then it is refused. */
  it("refuses a transfer raise that would take the wage cap below zero", () => {
    expect(canRaiseTransferBudget(500)).toBe(false);
    expect(canRaiseTransferBudget(1_000)).toBe(true);
  });
});
