import { render, screen } from "@testing-library/react";
import { describe, expect, it } from "vitest";

import type { ActiveLoanData, TeamData } from "../../store/gameStore";
import PlayerProfileLoanStatusBanner from "./PlayerProfileLoanStatusBanner";

const t = (key: string, options?: Record<string, unknown>) =>
  options && "fee" in options ? `${key}:${String(options.fee)}` : key;

const teams = [
  { id: "parent", name: "Parent FC" },
  { id: "borrower", name: "Borrower FC" },
] as unknown as TeamData[];

describe("PlayerProfileLoanStatusBanner", () => {
  /**
   * Given a loan with a buy option of €1,234,567
   * When the banner shows the loan's terms
   * Then the buy-option fee is exact, as on every other surface where it is negotiated.
   */
  it("shows the buy-option fee exactly", () => {
    const loan = {
      parent_team_id: "parent",
      loan_team_id: "borrower",
      start_date: "2026-08-01",
      end_date: "2027-06-30",
      wage_contribution_pct: 50,
      buy_option_fee: 1234567,
    } as ActiveLoanData;

    render(
      <PlayerProfileLoanStatusBanner
        loan={loan}
        teams={teams}
        managerTeamId="parent"
        language="en"
        t={t}
      />,
    );

    expect(screen.getByText("playerProfile.loanBuyOption:€1,234,567")).toBeInTheDocument();
  });
});
